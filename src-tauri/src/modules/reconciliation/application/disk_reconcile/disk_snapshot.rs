use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use crate::modules::workspace::domain::classifier::{
    classify_folder_strict_from_scan, scan_folder_strict, NodeType,
};
use crate::modules::workspace::domain::normalizer::{is_disabled_folder, normalize_display_name};

#[derive(Debug, Clone)]
pub struct DiskObjectEntry {
    pub folder_path: String,
    pub folder_path_key: String,
    pub name: String,
    pub is_disabled: bool,
    pub absolute_path: PathBuf,
    pub filesystem_identity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiskModEntry {
    pub folder_path: String,
    pub folder_path_key: String,
    pub object_folder_path_key: String,
    pub raw_name: String,
    pub absolute_path: PathBuf,
    pub filesystem_identity: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DiskProjection {
    pub objects: Vec<DiskObjectEntry>,
    pub mods: Vec<DiskModEntry>,
}

/// Directory names and normalized identities discovered without reading any
/// metadata or INI files. It is intentionally ephemeral: the filesystem stays
/// authoritative and this is rebuilt for each reconcile pass.
#[derive(Debug, Clone, Default)]
pub struct DiskIdentityCensus {
    pub entries: Vec<DiskIdentityCensusEntry>,
    pub top_level_roots: usize,
}

#[derive(Debug, Clone)]
pub struct DiskIdentityCensusEntry {
    pub folder_path: String,
    pub folder_path_key: String,
    pub raw_name: String,
    pub absolute_path: PathBuf,
}

impl DiskIdentityCensus {
    /// A collision spanning top-level roots can alter the object/mod ownership
    /// boundary. Classify the complete tree in that case rather than guessing
    /// which root owns the normalized identity from a partial projection.
    pub fn has_cross_root_ambiguity(&self) -> bool {
        let mut roots_by_identity = BTreeMap::<&str, BTreeSet<&str>>::new();
        for entry in &self.entries {
            let Some(root) = Path::new(&entry.folder_path)
                .components()
                .next()
                .and_then(|component| component.as_os_str().to_str())
            else {
                continue;
            };
            roots_by_identity
                .entry(&entry.folder_path_key)
                .or_default()
                .insert(root);
        }
        roots_by_identity.values().any(|roots| roots.len() > 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskDiscoveryScanCounts {
    pub census_directories: usize,
    pub classified_roots: usize,
    pub classified_directories: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiskDiscoveryTimings {
    pub census_ms: u64,
    pub classification_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DiskScopedDiscovery {
    pub census: DiskIdentityCensus,
    pub projection: DiskProjection,
    pub scoped: bool,
    pub scan_counts: DiskDiscoveryScanCounts,
    pub timings: DiskDiscoveryTimings,
}

/// A complete onboarding discovery derived from directory identity and strict
/// classification without a full asset metadata walk.
#[derive(Debug, Clone)]
pub struct OnboardingDiskDiscovery {
    pub discovery: DiskScopedDiscovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingDiscoveryPhase {
    Metadata,
    Classifying,
}

/// Progress from the filesystem preparation that precedes onboarding apply.
/// The caller owns event throttling because it has the session/game context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingDiscoveryProgress {
    pub phase: OnboardingDiscoveryPhase,
    pub completed_roots: usize,
    pub total_roots: usize,
    pub folders_classified: usize,
    pub current_root: Option<String>,
    pub is_terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskSnapshotProgress {
    pub completed_roots: usize,
    pub total_roots: usize,
    pub classified_directories: usize,
    pub current_root: Option<String>,
}

pub(crate) use crate::platform::fs::file_utils::filesystem_identity;

impl DiskProjection {
    pub fn scoped_to_roots(&self, changed_roots: &[String]) -> Self {
        let root_keys = changed_roots
            .iter()
            .map(|root| crate::shared::path_key::folder_path_key(root, None))
            .collect::<std::collections::HashSet<_>>();

        Self {
            objects: self
                .objects
                .iter()
                .filter(|entry| root_keys.contains(&entry.folder_path_key))
                .cloned()
                .collect(),
            mods: self
                .mods
                .iter()
                .filter(|entry| root_keys.contains(&entry.object_folder_path_key))
                .cloned()
                .collect(),
        }
    }
}

/// Why a snapshot failed. The caller degrades `SourceUnavailable` (drive
/// unplugged, folder removed mid-walk) to a no-op reconcile and surfaces
/// everything else as a hard error, so the distinction must not be carried
/// in message text.
#[derive(Debug, Clone)]
pub enum DiskProjectionError {
    SourceUnavailable(String),
    Failed(String),
}

impl DiskProjectionError {
    pub fn into_message(self) -> String {
        match self {
            Self::SourceUnavailable(message) | Self::Failed(message) => message,
        }
    }
}

type DiskProjectionResult<T> = Result<T, DiskProjectionError>;

fn list_runtime_dirs(path: &Path) -> DiskProjectionResult<Vec<PathBuf>> {
    let entries = std::fs::read_dir(path).map_err(|error| {
        DiskProjectionError::SourceUnavailable(format!(
            "Failed to read directory '{}': {error}",
            path.display()
        ))
    })?;
    let mut result = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            DiskProjectionError::SourceUnavailable(format!(
                "Failed to read directory entry in '{}': {error}",
                path.display()
            ))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            DiskProjectionError::SourceUnavailable(format!(
                "Failed to read file type for '{}' in '{}': {error}",
                entry.file_name().to_string_lossy(),
                path.display()
            ))
        })?;
        if !file_type.is_dir() {
            continue;
        }

        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        result.push(entry.path());
    }

    Ok(result)
}

/// `list_runtime_dirs` only yields `read_dir` entries, so every path has a name.
fn runtime_dir_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn relative_path_string(mods_path: &Path, path: &Path) -> DiskProjectionResult<String> {
    let relative = path.strip_prefix(mods_path).map_err(|error| {
        DiskProjectionError::Failed(format!(
            "Failed to compute relative path for '{}' under '{}': {error}",
            path.display(),
            mods_path.display()
        ))
    })?;
    Ok(relative.to_string_lossy().to_string())
}

/// Walk all directory names for normalized identity conflict detection without
/// invoking the strict classifier or reading INI files.
pub fn collect_disk_identity_census(mods_path: &Path) -> DiskProjectionResult<DiskIdentityCensus> {
    collect_disk_identity_census_with_progress(mods_path, None)
}

// Per-pass limit: normal reconciles hold the shared operation lock, while an
// onboarding snapshot refresh can run before acquiring that lock.
const CENSUS_ROOT_PARALLELISM: NonZeroUsize = NonZeroUsize::new(4).unwrap();
const NORMAL_CHILD_PARALLELISM: NonZeroUsize = NonZeroUsize::new(8).unwrap();
pub(super) const ONBOARDING_ROOT_PARALLELISM: NonZeroUsize = NonZeroUsize::new(8).unwrap();

fn collect_disk_identity_census_with_progress(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskIdentityCensus> {
    collect_disk_identity_census_with_parallelism(mods_path, progress, CENSUS_ROOT_PARALLELISM)
}

fn collect_census_root(
    mods_path: &Path,
    root_path: &Path,
) -> DiskProjectionResult<Vec<DiskIdentityCensusEntry>> {
    let mut entries = Vec::new();
    let mut pending = vec![root_path.to_path_buf()];
    while let Some(path) = pending.pop() {
        let folder_path = relative_path_string(mods_path, &path)?;
        entries.push(DiskIdentityCensusEntry {
            folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
            folder_path,
            raw_name: runtime_dir_name(&path),
            absolute_path: path.clone(),
        });
        pending.extend(list_runtime_dirs(&path)?);
    }
    Ok(entries)
}

fn sort_census_entries(entries: &mut [DiskIdentityCensusEntry]) {
    entries.sort_by_cached_key(|entry| entry.folder_path.to_ascii_lowercase());
}

fn merge_census_root_results(
    results: impl IntoIterator<Item = (usize, DiskProjectionResult<Vec<DiskIdentityCensusEntry>>)>,
    total_roots: usize,
    completed_root: &(impl Fn(usize) + Sync),
) -> DiskProjectionResult<Vec<DiskIdentityCensusEntry>> {
    let mut entries = Vec::new();
    let mut pending = BTreeMap::new();
    let mut completed_roots = 0;
    for (index, result) in results {
        debug_assert!(index < total_roots);
        let previous = pending.insert(index, result);
        debug_assert!(previous.is_none());
        while let Some(result) = pending.remove(&completed_roots) {
            entries.extend(result?);
            completed_roots += 1;
            completed_root(completed_roots);
        }
    }
    if completed_roots != total_roots {
        return Err(DiskProjectionError::Failed(format!(
            "Directory census stopped after {completed_roots} of {total_roots} root completions"
        )));
    }
    Ok(entries)
}

fn collect_census_roots(
    roots: &[PathBuf],
    parallelism: NonZeroUsize,
    read_root: &(impl Fn(&Path) -> DiskProjectionResult<Vec<DiskIdentityCensusEntry>> + Sync),
    completed_root: &(impl Fn(usize) + Sync),
) -> DiskProjectionResult<Vec<DiskIdentityCensusEntry>> {
    if roots.len() <= 1 || parallelism == NonZeroUsize::MIN {
        let results = roots
            .iter()
            .enumerate()
            .map(|(index, root)| (index, read_root(root)));
        return merge_census_root_results(results, roots.len(), completed_root);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(parallelism.get().min(roots.len()))
        .build_scoped(
            |thread| thread.run(),
            |pool| {
                pool.in_place_scope(|scope| {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    scope.spawn(move |_| {
                        // The receiver closes after the indexed merge selects an
                        // error. Stop remaining jobs without replacing that error.
                        let _receiver_disconnected = roots
                            .par_iter()
                            .enumerate()
                            .try_for_each_with(sender, |sender, (index, root)| {
                                sender.send((index, read_root(root)))
                            });
                    });
                    merge_census_root_results(receiver, roots.len(), completed_root)
                })
            },
        )
        .map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to start directory census workers: {error}"
            ))
        })?
}

fn collect_disk_identity_census_with_parallelism(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
    parallelism: NonZeroUsize,
) -> DiskProjectionResult<DiskIdentityCensus> {
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }

    let top_level_paths = list_runtime_dirs(mods_path)?;
    let total_roots = top_level_paths.len();
    if let Some(progress) = progress {
        progress(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots: 0,
            total_roots,
            folders_classified: 0,
            current_root: None,
            is_terminal: false,
        });
    }

    let mut entries = collect_census_roots(
        &top_level_paths,
        parallelism,
        &|root| collect_census_root(mods_path, root),
        &|completed_roots| {
            if let Some(progress) = progress {
                progress(OnboardingDiscoveryProgress {
                    phase: OnboardingDiscoveryPhase::Metadata,
                    completed_roots,
                    total_roots,
                    folders_classified: 0,
                    current_root: Some(runtime_dir_name(&top_level_paths[completed_roots - 1])),
                    is_terminal: completed_roots == total_roots,
                });
            }
        },
    )?;
    if total_roots == 0 {
        if let Some(progress) = progress {
            progress(OnboardingDiscoveryProgress {
                phase: OnboardingDiscoveryPhase::Metadata,
                completed_roots: 0,
                total_roots,
                folders_classified: 0,
                current_root: None,
                is_terminal: true,
            });
        }
    }
    sort_census_entries(&mut entries);

    Ok(DiskIdentityCensus {
        entries,
        top_level_roots: total_roots,
    })
}

/// Collect identity evidence only below roots already proven by a durable
/// internal mutation. External watcher input must keep using the global
/// census because it does not carry that proof.
fn collect_disk_identity_census_for_roots(
    mods_path: &Path,
    changed_roots: &[String],
) -> DiskProjectionResult<DiskIdentityCensus> {
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }

    let root_names = changed_roots.iter().collect::<BTreeSet<_>>();
    let mut pending = Vec::new();
    let mut top_level_roots = 0;
    for root_name in root_names {
        let mut components = Path::new(root_name).components();
        let valid_root = matches!(components.next(), Some(std::path::Component::Normal(_)))
            && components.next().is_none()
            && !root_name.starts_with('.');
        if !valid_root {
            return Err(DiskProjectionError::Failed(format!(
                "Trusted reconcile root is invalid: {root_name}"
            )));
        }

        let root_path = mods_path.join(root_name);
        if root_path.exists() && root_path.is_dir() {
            top_level_roots += 1;
            pending.push(root_path);
        }
    }

    let mut entries = Vec::new();
    while let Some(path) = pending.pop() {
        let folder_path = relative_path_string(mods_path, &path)?;
        let raw_name = runtime_dir_name(&path);
        entries.push(DiskIdentityCensusEntry {
            folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
            folder_path,
            raw_name,
            absolute_path: path.clone(),
        });
        pending.extend(list_runtime_dirs(&path)?);
    }
    sort_census_entries(&mut entries);

    Ok(DiskIdentityCensus {
        entries,
        top_level_roots,
    })
}

fn collect_terminal_mods(
    mods: &mut Vec<DiskModEntry>,
    mods_path: &Path,
    object_folder_path_key: &str,
    path: &Path,
    classified_directories: &AtomicUsize,
) -> DiskProjectionResult<()> {
    classified_directories.fetch_add(1, Ordering::Relaxed);
    let (node_type, children) = if path.is_dir() {
        let scan = scan_folder_strict(path)
            .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
        let (node_type, _reasons, _warnings) = classify_folder_strict_from_scan(&scan)
            .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
        (node_type, scan.child_dirs)
    } else {
        // The original classifier skipped missing/file paths; the subsequent
        // directory walk classified those failures as SourceUnavailable.
        (NodeType::ContainerFolder, list_runtime_dirs(path)?)
    };
    match node_type {
        NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
            let folder_path = relative_path_string(mods_path, path)?;
            let raw_name = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .ok_or_else(|| {
                    DiskProjectionError::Failed(format!(
                        "Disk Reconcile mod path has no name: {}",
                        path.display()
                    ))
                })?;
            let folder_path_key = crate::shared::path_key::folder_path_key(&folder_path, None);
            mods.push(DiskModEntry {
                folder_path: folder_path.clone(),
                folder_path_key,
                object_folder_path_key: object_folder_path_key.to_string(),
                raw_name,
                absolute_path: path.to_path_buf(),
                filesystem_identity: filesystem_identity(path),
            });
            Ok(())
        }
        NodeType::InternalAssets => Ok(()),
        NodeType::ContainerFolder => {
            for child_path in children {
                collect_terminal_mods(
                    mods,
                    mods_path,
                    object_folder_path_key,
                    &child_path,
                    classified_directories,
                )?;
            }
            Ok(())
        }
    }
}

pub fn collect_disk_projection(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
) -> DiskProjectionResult<DiskProjection> {
    collect_disk_projection_with_progress(mods_path, changed_roots, scoped, None)
}

pub fn collect_disk_projection_with_progress(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskProjection> {
    collect_disk_projection_with_progress_and_stats(mods_path, changed_roots, scoped, progress)
        .map(|(projection, _)| projection)
}

fn collect_disk_projection_with_progress_and_stats(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
) -> DiskProjectionResult<(DiskProjection, usize)> {
    collect_disk_projection_with_parallelism(
        mods_path,
        changed_roots,
        scoped,
        progress,
        NORMAL_CHILD_PARALLELISM,
    )
}

fn collect_terminal_mod_children(
    mods_path: &Path,
    object_folder_path_key: &str,
    children: &[PathBuf],
    classified_directories: &AtomicUsize,
    parallelism: NonZeroUsize,
) -> DiskProjectionResult<Vec<DiskModEntry>> {
    let mut mods = Vec::new();
    if parallelism == NonZeroUsize::MIN {
        for child in children {
            collect_terminal_mods(
                &mut mods,
                mods_path,
                object_folder_path_key,
                child,
                classified_directories,
            )?;
        }
    } else {
        for batch in children.chunks(parallelism.get()) {
            let branches = batch
                .par_iter()
                .map(|child| {
                    let mut branch = Vec::new();
                    collect_terminal_mods(
                        &mut branch,
                        mods_path,
                        object_folder_path_key,
                        child,
                        classified_directories,
                    )?;
                    Ok(branch)
                })
                .collect::<DiskProjectionResult<Vec<_>>>()?;
            for branch in branches {
                mods.extend(branch);
            }
        }
    }
    Ok(mods)
}

fn collect_disk_projection_with_parallelism(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
    parallelism: NonZeroUsize,
) -> DiskProjectionResult<(DiskProjection, usize)> {
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }

    let target_roots: Vec<(String, PathBuf)> = if scoped {
        changed_roots
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|root| {
                let path = mods_path.join(&root);
                (root, path)
            })
            .collect()
    } else {
        list_runtime_dirs(mods_path)?
            .into_iter()
            .map(|path| (runtime_dir_name(&path), path))
            .collect()
    };

    // Each root is an independent walk whose per-mod classify pass reads a
    // directory and every ini in it. `par_iter` keeps input order, so the
    // projection lands in the same sequence the serial loop produced.
    let total_roots = target_roots.len();
    // Existing root parallelism is retained; only spare child slots are distributed.
    let child_parallelism =
        NonZeroUsize::new(parallelism.get() / total_roots.max(1)).unwrap_or(NonZeroUsize::MIN);
    if let Some(progress) = progress {
        progress(DiskSnapshotProgress {
            completed_roots: 0,
            total_roots,
            classified_directories: 0,
            current_root: None,
        });
    }
    let completed_roots = AtomicUsize::new(0);
    let classified_directories = AtomicUsize::new(0);
    let per_root: Vec<Option<(DiskObjectEntry, Vec<DiskModEntry>)>> = target_roots
        .par_iter()
        .map(|(root_name, root_path)| {
            if !root_path.exists() || !root_path.is_dir() {
                let completed = completed_roots.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(progress) = progress {
                    progress(DiskSnapshotProgress {
                        completed_roots: completed,
                        total_roots,
                        classified_directories: classified_directories.load(Ordering::Relaxed),
                        current_root: Some(root_name.clone()),
                    });
                }
                return Ok(None);
            }

            let object_entry = DiskObjectEntry {
                folder_path: root_name.clone(),
                folder_path_key: crate::shared::path_key::folder_path_key(root_name, None),
                name: normalize_display_name(root_name).into_owned(),
                is_disabled: is_disabled_folder(root_name),
                absolute_path: root_path.clone(),
                filesystem_identity: filesystem_identity(root_path),
            };

            let mods = collect_terminal_mod_children(
                mods_path,
                &object_entry.folder_path_key,
                &list_runtime_dirs(root_path)?,
                &classified_directories,
                child_parallelism,
            )?;

            let entry = Ok(Some((object_entry, mods)));
            let completed = completed_roots.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(progress) = progress {
                progress(DiskSnapshotProgress {
                    completed_roots: completed,
                    total_roots,
                    classified_directories: classified_directories.load(Ordering::Relaxed),
                    current_root: Some(root_name.clone()),
                });
            }
            entry
        })
        .collect::<DiskProjectionResult<Vec<_>>>()?;

    let mut projection = DiskProjection::default();
    for (object_entry, mods) in per_root.into_iter().flatten() {
        projection.objects.push(object_entry);
        projection.mods.extend(mods);
    }

    Ok((projection, classified_directories.load(Ordering::Relaxed)))
}

/// Collect a one-pass directory census plus a strict projection. The census
/// first determines whether a scoped input is unsafe; the selected projection
/// is then classified exactly once.
pub fn collect_scoped_disk_discovery_with_progress(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskScopedDiscovery> {
    collect_scoped_disk_discovery_with_census(mods_path, changed_roots, scoped, None, progress)
}

/// Trusted internal mutations have already validated every source,
/// destination, sibling collision, and filesystem identity under the game
/// mutation lease. Their terminal projection can therefore avoid observing
/// unrelated roots entirely.
pub fn collect_trusted_scoped_disk_discovery_with_progress(
    mods_path: &Path,
    changed_roots: &[String],
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskScopedDiscovery> {
    if changed_roots.is_empty() {
        return Err(DiskProjectionError::Failed(
            "Trusted scoped reconcile requires at least one changed root".to_string(),
        ));
    }
    let census_started = std::time::Instant::now();
    let census = collect_disk_identity_census_for_roots(mods_path, changed_roots)?;
    let census_ms = census_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let classification_started = std::time::Instant::now();
    let (projection, classified_directories) =
        collect_disk_projection_with_progress_and_stats(mods_path, changed_roots, true, progress)?;
    let classification_ms = classification_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    Ok(DiskScopedDiscovery {
        scan_counts: DiskDiscoveryScanCounts {
            census_directories: census.entries.len(),
            classified_roots: changed_roots.iter().collect::<BTreeSet<_>>().len(),
            classified_directories,
        },
        timings: DiskDiscoveryTimings {
            census_ms,
            classification_ms,
        },
        census,
        projection,
        scoped: true,
    })
}

fn collect_scoped_disk_discovery_with_census(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
    precomputed_census: Option<DiskIdentityCensus>,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskScopedDiscovery> {
    let census_started = std::time::Instant::now();
    let census = match precomputed_census {
        Some(census) => census,
        None => collect_disk_identity_census(mods_path)?,
    };
    let census_ms = census_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let scoped = scoped && !census.has_cross_root_ambiguity();
    let classification_started = std::time::Instant::now();
    let (projection, classified_directories) = collect_disk_projection_with_progress_and_stats(
        mods_path,
        changed_roots,
        scoped,
        progress,
    )?;
    let classification_ms = classification_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let classified_roots = if scoped {
        changed_roots.iter().collect::<BTreeSet<_>>().len()
    } else {
        census.top_level_roots
    };

    Ok(DiskScopedDiscovery {
        scan_counts: DiskDiscoveryScanCounts {
            census_directories: census.entries.len(),
            classified_roots,
            classified_directories,
        },
        timings: DiskDiscoveryTimings {
            census_ms,
            classification_ms,
        },
        census,
        projection,
        scoped,
    })
}

/// Build the directory census and strict terminal-mod projection without
/// reading every asset's metadata. First-run indexing no longer computes
/// storage statistics for the dashboard.
pub fn collect_onboarding_disk_discovery(
    mods_path: &Path,
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    collect_onboarding_disk_discovery_with_progress(mods_path, None)
}

pub fn collect_onboarding_disk_discovery_with_progress(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    collect_onboarding_disk_discovery_with_progress_and_gate(mods_path, progress, None, None)
}

/// The gate is held only while a small batch of roots is classified. This
/// lets another game's foreground scan take over without discarding roots
/// that were already completed.
pub fn collect_onboarding_disk_discovery_with_progress_and_gate(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
    gate: Option<&(dyn Fn() -> DiskProjectionResult<Box<dyn Send>> + Send + Sync)>,
    cancelled: Option<&AtomicBool>,
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    collect_onboarding_disk_discovery_with_parallelism(
        mods_path,
        progress,
        gate,
        cancelled,
        ONBOARDING_ROOT_PARALLELISM,
    )
}

pub(super) fn collect_onboarding_disk_discovery_with_parallelism(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
    gate: Option<&(dyn Fn() -> DiskProjectionResult<Box<dyn Send>> + Send + Sync)>,
    cancelled: Option<&AtomicBool>,
    parallelism: NonZeroUsize,
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    check_onboarding_cancelled(cancelled)?;
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }

    let top_level_list_started = std::time::Instant::now();
    let top_level_paths = list_runtime_dirs(mods_path)?;
    let mut census_duration = top_level_list_started.elapsed();
    let total_roots = top_level_paths.len();
    if let Some(progress) = progress {
        progress(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots: 0,
            total_roots,
            folders_classified: 0,
            current_root: None,
            is_terminal: false,
        });
        progress(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Classifying,
            completed_roots: 0,
            total_roots,
            folders_classified: 0,
            current_root: None,
            is_terminal: total_roots == 0,
        });
    }

    let completed_roots = AtomicUsize::new(0);
    let classified_directories = AtomicUsize::new(0);
    let mut roots = Vec::with_capacity(total_roots);
    for batch in top_level_paths.chunks(parallelism.get()) {
        check_onboarding_cancelled(cancelled)?;
        let _permit = gate.map(|acquire| acquire()).transpose()?;
        let child_parallelism = parallelism.get() / batch.len();
        let scanned = batch
            .par_iter()
            .map(|root_path| {
                let root_name = runtime_dir_name(root_path);
                let last_reported_at = Mutex::new(None);
                let activity = OnboardingTraversalProgress {
                    progress,
                    cancelled,
                    completed_roots: &completed_roots,
                    classified_directories: &classified_directories,
                    total_roots,
                    root_name: &root_name,
                    last_reported_at: &last_reported_at,
                };
                let root = collect_onboarding_root_discovery(
                    mods_path,
                    root_path,
                    &classified_directories,
                    &activity,
                    child_parallelism,
                )?;
                check_onboarding_cancelled(cancelled)?;
                let completed = completed_roots.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(progress) = progress {
                    progress(OnboardingDiscoveryProgress {
                        phase: OnboardingDiscoveryPhase::Classifying,
                        completed_roots: completed,
                        total_roots,
                        folders_classified: classified_directories.load(Ordering::Relaxed),
                        current_root: Some(runtime_dir_name(root_path)),
                        is_terminal: completed == total_roots,
                    });
                }
                Ok(root)
            })
            .collect::<DiskProjectionResult<Vec<_>>>()?;
        roots.extend(scanned);
    }

    let mut census_entries = Vec::new();
    let mut projection = DiskProjection::default();
    let mut classification_duration = Duration::ZERO;
    for root in roots {
        census_duration += root.branch.census_duration;
        classification_duration += root.branch.classification_duration;
        census_entries.extend(root.branch.census_entries);
        projection.objects.push(root.object);
        projection.mods.extend(root.branch.mods);
    }
    sort_census_entries(&mut census_entries);
    let census_directories = census_entries.len();

    Ok(OnboardingDiskDiscovery {
        discovery: DiskScopedDiscovery {
            census: DiskIdentityCensus {
                entries: census_entries,
                top_level_roots: total_roots,
            },
            projection,
            scoped: false,
            scan_counts: DiskDiscoveryScanCounts {
                census_directories,
                classified_roots: total_roots,
                classified_directories: classified_directories.load(Ordering::Relaxed),
            },
            timings: DiskDiscoveryTimings {
                census_ms: duration_ms(census_duration),
                classification_ms: duration_ms(classification_duration),
            },
        },
    })
}

struct OnboardingRootDiscovery {
    object: DiskObjectEntry,
    branch: OnboardingBranchDiscovery,
}

#[derive(Default)]
struct OnboardingBranchDiscovery {
    census_entries: Vec<DiskIdentityCensusEntry>,
    mods: Vec<DiskModEntry>,
    census_duration: Duration,
    classification_duration: Duration,
}

fn check_onboarding_cancelled(cancelled: Option<&AtomicBool>) -> DiskProjectionResult<()> {
    if cancelled.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        Err(DiskProjectionError::Failed(
            "Onboarding indexing was cancelled".to_string(),
        ))
    } else {
        Ok(())
    }
}

struct OnboardingTraversalProgress<'a> {
    progress: Option<&'a (dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
    cancelled: Option<&'a AtomicBool>,
    completed_roots: &'a AtomicUsize,
    classified_directories: &'a AtomicUsize,
    total_roots: usize,
    root_name: &'a str,
    last_reported_at: &'a Mutex<Option<Instant>>,
}

impl OnboardingTraversalProgress<'_> {
    fn visit(&self) -> DiskProjectionResult<()> {
        check_onboarding_cancelled(self.cancelled)?;
        if self.progress.is_none() {
            return Ok(());
        }
        let mut last_reported_at = self.last_reported_at.lock().map_err(|_| {
            DiskProjectionError::Failed("Onboarding progress lock was poisoned".to_string())
        })?;
        if last_reported_at.is_some_and(|last| last.elapsed() < Duration::from_millis(250)) {
            return Ok(());
        }
        if let Some(progress) = self.progress {
            progress(OnboardingDiscoveryProgress {
                phase: OnboardingDiscoveryPhase::Classifying,
                completed_roots: self.completed_roots.load(Ordering::Relaxed),
                total_roots: self.total_roots,
                folders_classified: self.classified_directories.load(Ordering::Relaxed),
                current_root: Some(self.root_name.to_string()),
                is_terminal: false,
            });
        }
        *last_reported_at = Some(Instant::now());
        check_onboarding_cancelled(self.cancelled)
    }
}

fn collect_onboarding_root_discovery(
    mods_path: &Path,
    root_path: &Path,
    classified_directories: &AtomicUsize,
    activity: &OnboardingTraversalProgress<'_>,
    child_parallelism: usize,
) -> DiskProjectionResult<OnboardingRootDiscovery> {
    check_onboarding_cancelled(activity.cancelled)?;
    let root_name = runtime_dir_name(root_path);
    let object = DiskObjectEntry {
        folder_path: root_name.clone(),
        folder_path_key: crate::shared::path_key::folder_path_key(&root_name, None),
        name: normalize_display_name(&root_name).into_owned(),
        is_disabled: is_disabled_folder(&root_name),
        absolute_path: root_path.to_path_buf(),
        filesystem_identity: filesystem_identity(root_path),
    };
    let mut result = OnboardingRootDiscovery {
        object,
        branch: OnboardingBranchDiscovery {
            census_entries: vec![census_entry(mods_path, root_path)?],
            ..Default::default()
        },
    };
    let object_folder_path_key = result.object.folder_path_key.clone();
    let root_children = timed_runtime_dirs(root_path, &mut result.branch.census_duration)?;
    if child_parallelism == 1 {
        for child_path in root_children {
            collect_onboarding_folder(
                &child_path,
                mods_path,
                &object_folder_path_key,
                &mut result.branch,
                classified_directories,
                activity,
            )?;
        }
    } else {
        // Spare root slots are shared by immediate children; recursion stays serial.
        for batch in root_children.chunks(child_parallelism) {
            check_onboarding_cancelled(activity.cancelled)?;
            let branches = batch
                .par_iter()
                .map(|child_path| {
                    let mut branch = OnboardingBranchDiscovery::default();
                    collect_onboarding_folder(
                        child_path,
                        mods_path,
                        &object_folder_path_key,
                        &mut branch,
                        classified_directories,
                        activity,
                    )?;
                    Ok(branch)
                })
                .collect::<DiskProjectionResult<Vec<_>>>()?;
            for branch in branches {
                result.branch.census_entries.extend(branch.census_entries);
                result.branch.mods.extend(branch.mods);
                result.branch.census_duration += branch.census_duration;
                result.branch.classification_duration += branch.classification_duration;
            }
        }
    }
    Ok(result)
}

fn collect_onboarding_folder(
    path: &Path,
    mods_path: &Path,
    object_folder_path_key: &str,
    result: &mut OnboardingBranchDiscovery,
    classified_directories: &AtomicUsize,
    activity: &OnboardingTraversalProgress<'_>,
) -> DiskProjectionResult<()> {
    check_onboarding_cancelled(activity.cancelled)?;
    result.census_entries.push(census_entry(mods_path, path)?);
    let scan_started = std::time::Instant::now();
    let scan =
        scan_folder_strict(path).map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
    result.census_duration += scan_started.elapsed();
    let classification_started = std::time::Instant::now();
    let (node_type, _reasons, _warnings) = classify_folder_strict_from_scan(&scan)
        .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
    result.classification_duration += classification_started.elapsed();
    classified_directories.fetch_add(1, Ordering::Relaxed);
    activity.visit()?;

    let children = scan.child_dirs;
    match node_type {
        NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
            result.mods.push(onboarding_mod_entry(
                mods_path,
                object_folder_path_key,
                path,
            )?);
            for child_path in children {
                collect_onboarding_census_only(&child_path, mods_path, result, activity)?;
            }
        }
        NodeType::InternalAssets => {
            for child_path in children {
                collect_onboarding_census_only(&child_path, mods_path, result, activity)?;
            }
        }
        NodeType::ContainerFolder => {
            for child_path in children {
                collect_onboarding_folder(
                    &child_path,
                    mods_path,
                    object_folder_path_key,
                    result,
                    classified_directories,
                    activity,
                )?;
            }
        }
    }
    Ok(())
}

fn collect_onboarding_census_only(
    path: &Path,
    mods_path: &Path,
    result: &mut OnboardingBranchDiscovery,
    activity: &OnboardingTraversalProgress<'_>,
) -> DiskProjectionResult<()> {
    activity.visit()?;
    result.census_entries.push(census_entry(mods_path, path)?);
    for child_path in timed_runtime_dirs(path, &mut result.census_duration)? {
        collect_onboarding_census_only(&child_path, mods_path, result, activity)?;
    }
    Ok(())
}

fn census_entry(mods_path: &Path, path: &Path) -> DiskProjectionResult<DiskIdentityCensusEntry> {
    let folder_path = relative_path_string(mods_path, path)?;
    Ok(DiskIdentityCensusEntry {
        folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
        raw_name: runtime_dir_name(path),
        absolute_path: path.to_path_buf(),
        folder_path,
    })
}

fn timed_runtime_dirs(path: &Path, duration: &mut Duration) -> DiskProjectionResult<Vec<PathBuf>> {
    let started = std::time::Instant::now();
    let dirs = list_runtime_dirs(path)?;
    *duration += started.elapsed();
    Ok(dirs)
}

fn onboarding_mod_entry(
    mods_path: &Path,
    object_folder_path_key: &str,
    path: &Path,
) -> DiskProjectionResult<DiskModEntry> {
    let folder_path = relative_path_string(mods_path, path)?;
    let raw_name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| {
            DiskProjectionError::Failed(format!(
                "Disk Reconcile mod path has no name: {}",
                path.display()
            ))
        })?;
    Ok(DiskModEntry {
        folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
        folder_path,
        object_folder_path_key: object_folder_path_key.to_string(),
        raw_name,
        absolute_path: path.to_path_buf(),
        filesystem_identity: filesystem_identity(path),
    })
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

/// Rebuild only the changed onboarding roots. The global directory census is
/// still collected by scoped discovery to retain conflict correctness.
pub fn collect_scoped_onboarding_disk_discovery(
    mods_path: &Path,
    changed_roots: &[String],
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    let discovery =
        collect_scoped_disk_discovery_with_progress(mods_path, changed_roots, true, None)?;
    Ok(OnboardingDiskDiscovery { discovery })
}

#[cfg(test)]
#[path = "census_tests.rs"]
mod census_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_snapshot_skips_container_only_folders_and_indexes_terminal_mod_roots() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let object_root = temp.path().join("Alice");
        let container = object_root.join("Nested");
        let terminal = container.join("Blue Dress");
        let empty_container = object_root.join("Empty Container");

        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::create_dir_all(&empty_container).expect("empty container should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash = abc\n",
        )
        .expect("ini should be written");
        std::fs::write(terminal.join("mesh.buf"), "mesh").expect("asset should be written");

        let projection =
            collect_disk_projection(temp.path(), &[], false).expect("snapshot should succeed");

        assert_eq!(projection.objects.len(), 1);
        assert_eq!(projection.objects[0].folder_path, "Alice");
        assert_eq!(projection.mods.len(), 1);
        assert_eq!(
            projection.mods[0].folder_path,
            PathBuf::from("Alice")
                .join("Nested")
                .join("Blue Dress")
                .to_string_lossy()
                .to_string()
        );
    }

    #[test]
    fn onboarding_discovery_classifies_mods_beside_root_files() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let terminal = temp.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(temp.path().join("d3dx.ini"), b"abc")
            .expect("direct root file should be written");
        let ini_bytes = b"[TextureOverrideTest]\nhash = abc\n";
        std::fs::write(terminal.join("mod.ini"), ini_bytes).expect("mod ini should be written");
        std::fs::write(terminal.join("mesh.buf"), b"mesh").expect("mod asset should be written");

        let onboarding = collect_onboarding_disk_discovery(temp.path())
            .expect("onboarding discovery should succeed");
        assert_eq!(onboarding.discovery.projection.mods.len(), 1);
    }

    #[test]
    fn onboarding_discovery_ignores_direct_files_and_hidden_folders_without_sizing() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let terminal = temp.path().join("Alice").join("Nested").join("Blue Dress");
        let hidden_terminal = temp.path().join(".cache").join("Ignored Mod");
        std::fs::create_dir_all(&terminal).expect("nested terminal folder should be created");
        std::fs::create_dir_all(&hidden_terminal)
            .expect("hidden terminal folder should be created");
        std::fs::write(temp.path().join("d3dx.ini"), b"abc")
            .expect("direct root file should be written");
        let ini_bytes = b"[TextureOverrideTest]\nhash = abc\n";
        std::fs::write(terminal.join("mod.ini"), ini_bytes)
            .expect("nested mod ini should be written");
        std::fs::write(terminal.join("mesh.buf"), b"asset")
            .expect("nested asset should be written");
        std::fs::write(hidden_terminal.join("mod.ini"), b"hidden")
            .expect("hidden mod ini should be written");

        let census =
            collect_disk_identity_census(temp.path()).expect("identity census should succeed");
        assert!(census
            .entries
            .iter()
            .all(|entry| !entry.folder_path.starts_with(".cache")));

        let onboarding = collect_onboarding_disk_discovery(temp.path())
            .expect("onboarding discovery should succeed");
        assert_eq!(onboarding.discovery.projection.mods.len(), 1);
    }

    #[test]
    fn fused_onboarding_discovery_matches_legacy_census_and_projection() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let blue_dress = temp.path().join("Alice").join("Nested").join("Blue Dress");
        let red_dress = temp.path().join("Bob").join("Red Dress");
        let hidden = temp.path().join("Alice").join(".cache").join("Ignored Mod");
        let nested_asset = blue_dress.join("Assets").join("Textures");
        std::fs::create_dir_all(&nested_asset).expect("nested asset folder should be created");
        std::fs::create_dir_all(&red_dress).expect("second terminal folder should be created");
        std::fs::create_dir_all(&hidden).expect("hidden folder should be created");
        std::fs::create_dir_all(temp.path().join("Alice").join("Empty Container"))
            .expect("empty container should be created");
        std::fs::write(temp.path().join("d3dx.ini"), b"direct root file")
            .expect("direct file should be written");
        for terminal in [&blue_dress, &red_dress] {
            std::fs::write(
                terminal.join("mod.ini"),
                "[TextureOverrideTest]\nhash = abc\n",
            )
            .expect("mod ini should be written");
        }
        std::fs::write(nested_asset.join("diffuse.dds"), b"asset")
            .expect("asset should be written");
        std::fs::write(hidden.join("mod.ini"), b"hidden").expect("hidden ini should be written");

        let legacy_census = collect_disk_identity_census(temp.path()).expect("legacy census");
        let (legacy_projection, legacy_classified_directories) =
            collect_disk_projection_with_progress_and_stats(temp.path(), &[], false, None)
                .expect("legacy projection");
        let fused = collect_onboarding_disk_discovery(temp.path())
            .expect("fused onboarding discovery")
            .discovery;

        let census_shape = |census: &DiskIdentityCensus| {
            census
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.folder_path.clone(),
                        entry.folder_path_key.clone(),
                        entry.raw_name.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let object_shape = |projection: &DiskProjection| {
            projection
                .objects
                .iter()
                .map(|entry| {
                    (
                        entry.folder_path.clone(),
                        entry.folder_path_key.clone(),
                        entry.name.clone(),
                        entry.is_disabled,
                    )
                })
                .collect::<Vec<_>>()
        };
        let mod_shape = |projection: &DiskProjection| {
            projection
                .mods
                .iter()
                .map(|entry| {
                    (
                        entry.folder_path.clone(),
                        entry.folder_path_key.clone(),
                        entry.object_folder_path_key.clone(),
                        entry.raw_name.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(census_shape(&fused.census), census_shape(&legacy_census));
        assert_eq!(
            object_shape(&fused.projection),
            object_shape(&legacy_projection)
        );
        assert_eq!(mod_shape(&fused.projection), mod_shape(&legacy_projection));
        assert_eq!(
            fused.scan_counts.census_directories,
            legacy_census.entries.len()
        );
        assert_eq!(
            fused.scan_counts.classified_directories,
            legacy_classified_directories
        );
        assert!(!fused.census.has_cross_root_ambiguity());
        assert!(fused.census.entries.iter().any(|entry| {
            entry.folder_path
                == PathBuf::from("Alice")
                    .join("Nested")
                    .join("Blue Dress")
                    .join("Assets")
                    .join("Textures")
                    .to_string_lossy()
        }));
        assert!(fused
            .census
            .entries
            .iter()
            .all(|entry| !entry.folder_path.contains(".cache")));
    }

    #[test]
    fn identity_census_detects_conflicting_normalized_names_across_roots() {
        let census = DiskIdentityCensus {
            entries: vec![
                DiskIdentityCensusEntry {
                    folder_path: "Alice/Shared".to_string(),
                    folder_path_key: "shared".to_string(),
                    raw_name: "Shared".to_string(),
                    absolute_path: PathBuf::from("Alice/Shared"),
                },
                DiskIdentityCensusEntry {
                    folder_path: "Bob/shared".to_string(),
                    folder_path_key: "shared".to_string(),
                    raw_name: "shared".to_string(),
                    absolute_path: PathBuf::from("Bob/shared"),
                },
            ],
            top_level_roots: 2,
        };

        assert!(census.has_cross_root_ambiguity());
    }

    #[test]
    fn scoped_onboarding_discovery_scans_only_changed_roots_without_sizing() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        for root in ["Alice", "Bob"] {
            let terminal = temp.path().join(root).join("Blue Dress");
            std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
            std::fs::write(
                terminal.join("mod.ini"),
                "[TextureOverrideTest]\nhash = abc\n",
            )
            .expect("mod ini should be written");
        }

        let onboarding =
            collect_scoped_onboarding_disk_discovery(temp.path(), &["Bob".to_string()])
                .expect("scoped onboarding discovery should succeed");

        assert!(onboarding.discovery.scoped);
        assert_eq!(onboarding.discovery.projection.objects.len(), 1);
        assert_eq!(
            onboarding.discovery.projection.objects[0].folder_path,
            "Bob"
        );
    }

    #[test]
    fn scoped_projection_is_derived_from_the_same_full_snapshot() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        for object in ["Alice", "Bob"] {
            let terminal = temp.path().join(object).join("Blue Dress");
            std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
            std::fs::write(
                terminal.join("mod.ini"),
                "[TextureOverrideTest]\nhash = abc\n",
            )
            .expect("ini should be written");
        }

        let full = collect_disk_projection(temp.path(), &[], false).expect("full snapshot");
        let scoped = full.scoped_to_roots(&["alice".to_string()]);

        assert_eq!(full.objects.len(), 2);
        assert_eq!(scoped.objects.len(), 1);
        assert_eq!(scoped.objects[0].folder_path, "Alice");
        assert_eq!(scoped.mods.len(), 1);
        assert_eq!(scoped.mods[0].object_folder_path_key, "alice");
    }

    #[test]
    fn scoped_discovery_classifies_only_changed_root_after_global_name_census() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        for object in ["Alice", "Bob", "Carol"] {
            let terminal = temp.path().join(object).join("Blue Dress");
            std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
            std::fs::write(
                terminal.join("mod.ini"),
                "[TextureOverrideTest]\nhash = abc\n",
            )
            .expect("ini should be written");
        }

        let discovery = collect_scoped_disk_discovery_with_progress(
            temp.path(),
            &["Alice".to_string()],
            true,
            None,
        )
        .expect("scoped discovery should succeed");

        assert!(discovery.scoped);
        assert_eq!(discovery.scan_counts.classified_roots, 1);
        assert_eq!(discovery.scan_counts.census_directories, 6);
        assert_eq!(discovery.scan_counts.classified_directories, 1);
        assert_eq!(discovery.projection.objects.len(), 1);
        assert_eq!(discovery.projection.mods.len(), 1);
    }

    #[test]
    fn trusted_scoped_discovery_does_not_census_unrelated_roots() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        for object in ["Alice", "Bob", "Carol"] {
            let terminal = temp.path().join(object).join("Blue Dress");
            std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
            std::fs::write(
                terminal.join("mod.ini"),
                "[TextureOverrideTest]\nhash = abc\n",
            )
            .expect("ini should be written");
        }

        let discovery = collect_trusted_scoped_disk_discovery_with_progress(
            temp.path(),
            &["Alice".to_string()],
            None,
        )
        .expect("trusted scoped discovery should succeed");

        assert!(discovery.scoped);
        assert_eq!(discovery.scan_counts.classified_roots, 1);
        assert_eq!(discovery.scan_counts.census_directories, 2);
        assert_eq!(discovery.scan_counts.classified_directories, 1);
        assert_eq!(discovery.projection.objects.len(), 1);
        assert_eq!(discovery.projection.objects[0].folder_path, "Alice");
    }

    #[test]
    fn snapshot_rejects_lossy_ini_bytes_instead_of_silently_skipping_the_mod() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let terminal = temp.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(terminal.join("mod.ini"), [0xFF, 0xFE, 0xFD])
            .expect("invalid ini bytes should be written");

        let error = collect_disk_projection(temp.path(), &[], false)
            .expect_err("authoritative snapshot must reject lossy INI bytes");

        assert!(
            matches!(error, DiskProjectionError::Failed(message) if message.contains("encoding"))
        );
    }

    #[test]
    #[ignore = "manual 100/1k/10k-folder performance fixtures"]
    fn benchmark_disk_snapshot_fixtures() {
        fn percentile(
            samples: &mut [std::time::Duration],
            percentile: usize,
        ) -> std::time::Duration {
            samples.sort_unstable();
            let rank = (samples.len() * percentile).div_ceil(100).saturating_sub(1);
            samples[rank]
        }

        for total_mods in [100_usize, 1_000, 10_000] {
            let temp = tempfile::tempdir().expect("tempdir should be created");
            let mods_per_object = 100;
            let objects = total_mods.div_ceil(mods_per_object);
            for object_index in 0..objects {
                let count = (total_mods - object_index * mods_per_object).min(mods_per_object);
                for mod_index in 0..count {
                    let terminal = temp
                        .path()
                        .join(format!("Object {object_index:03}"))
                        .join(format!("Mod {mod_index:03}"));
                    std::fs::create_dir_all(&terminal).expect("benchmark folder");
                    std::fs::write(
                        terminal.join("mod.ini"),
                        "[TextureOverrideBenchmark]\nhash = abc\n",
                    )
                    .expect("benchmark ini");
                }
            }

            let started = std::time::Instant::now();
            let cold_full =
                collect_scoped_disk_discovery_with_progress(temp.path(), &[], false, None)
                    .expect("full snapshot should succeed");
            let cold_full_elapsed = started.elapsed();
            let started = std::time::Instant::now();
            let cold_onboarding = collect_onboarding_disk_discovery(temp.path())
                .expect("onboarding discovery should succeed");
            let cold_onboarding_elapsed = started.elapsed();
            let started = std::time::Instant::now();
            let cold_scoped = collect_trusted_scoped_disk_discovery_with_progress(
                temp.path(),
                &["Object 000".to_string()],
                None,
            )
            .expect("trusted scoped snapshot should succeed");
            let cold_scoped_elapsed = started.elapsed();

            let mut warm_full = Vec::new();
            let mut warm_onboarding = Vec::new();
            let mut warm_scoped = Vec::new();
            for _ in 0..5 {
                let started = std::time::Instant::now();
                collect_scoped_disk_discovery_with_progress(temp.path(), &[], false, None)
                    .expect("warm full snapshot should succeed");
                warm_full.push(started.elapsed());
                let started = std::time::Instant::now();
                collect_onboarding_disk_discovery(temp.path())
                    .expect("warm onboarding discovery should succeed");
                warm_onboarding.push(started.elapsed());
                let started = std::time::Instant::now();
                collect_trusted_scoped_disk_discovery_with_progress(
                    temp.path(),
                    &["Object 000".to_string()],
                    None,
                )
                .expect("warm trusted scoped snapshot should succeed");
                warm_scoped.push(started.elapsed());
            }

            assert_eq!(cold_full.projection.mods.len(), total_mods);
            assert_eq!(cold_onboarding.discovery.projection.mods.len(), total_mods);
            assert_eq!(
                cold_onboarding.discovery.scan_counts.census_directories,
                cold_full.scan_counts.census_directories
            );
            assert_eq!(cold_full.scan_counts.classified_directories, total_mods);
            assert_eq!(
                cold_scoped.scan_counts.classified_directories,
                total_mods.min(mods_per_object)
            );
            assert!(cold_scoped.scoped);
            let full_p50 = percentile(&mut warm_full.clone(), 50);
            let full_p95 = percentile(&mut warm_full, 95);
            let onboarding_p50 = percentile(&mut warm_onboarding.clone(), 50);
            let onboarding_p95 = percentile(&mut warm_onboarding, 95);
            let scoped_p50 = percentile(&mut warm_scoped.clone(), 50);
            let scoped_p95 = percentile(&mut warm_scoped, 95);
            eprintln!(
                "disk_snapshot_{total_mods}: legacy_first_full={cold_full_elapsed:?} onboarding_first={cold_onboarding_elapsed:?} first_scoped={cold_scoped_elapsed:?} legacy_warm_full_p50={full_p50:?} legacy_warm_full_p95={full_p95:?} onboarding_warm_p50={onboarding_p50:?} onboarding_warm_p95={onboarding_p95:?} warm_scoped_p50={scoped_p50:?} warm_scoped_p95={scoped_p95:?} scoped_census={} scoped_classified={}",
                cold_scoped.scan_counts.census_directories,
                cold_scoped.scan_counts.classified_directories,
            );
        }
    }
}
