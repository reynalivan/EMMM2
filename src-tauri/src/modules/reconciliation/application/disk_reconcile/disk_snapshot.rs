use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::modules::workspace::domain::classifier::{classify_folder_strict, NodeType};
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
    /// `None` when an incremental reconcile did not need to rescan this mod.
    pub size_bytes: Option<i64>,
}

/// Restricts size metadata walks to new or changed terminal mods. The initial
/// backfill opts into every terminal mod once, rather than taxing every scan.
#[derive(Debug, Clone, Default)]
pub struct DiskSizeScan {
    scan_all: bool,
    known_mod_keys: HashSet<String>,
    changed_path_keys: Vec<String>,
    precomputed_sizes: Option<HashMap<String, i64>>,
}

impl DiskSizeScan {
    pub fn full() -> Self {
        Self {
            scan_all: true,
            ..Self::default()
        }
    }

    pub fn incremental(
        mods_path: &Path,
        known_mod_keys: HashSet<String>,
        changed_paths: &[String],
    ) -> Self {
        let changed_path_keys = changed_paths
            .iter()
            .filter_map(|value| Path::new(value).strip_prefix(mods_path).ok())
            .filter(|relative| !relative.as_os_str().is_empty())
            .map(|relative| {
                crate::shared::path_key::folder_path_key(&relative.to_string_lossy(), None)
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            scan_all: changed_paths
                .iter()
                .any(|value| Path::new(value) == mods_path),
            known_mod_keys,
            changed_path_keys,
            precomputed_sizes: None,
        }
    }

    pub fn precomputed(directory_sizes: HashMap<String, i64>) -> Self {
        Self {
            scan_all: true,
            precomputed_sizes: Some(directory_sizes),
            ..Self::default()
        }
    }

    fn should_measure(&self, mod_key: &str) -> bool {
        self.scan_all
            || !self.known_mod_keys.contains(mod_key)
            || self
                .changed_path_keys
                .iter()
                .any(|changed_key| path_key_is_ancestor_or_descendant(mod_key, changed_key))
    }

    fn size_for(&self, mod_key: &str, path: &Path) -> DiskProjectionResult<Option<i64>> {
        if let Some(sizes) = &self.precomputed_sizes {
            return Ok(Some(*sizes.get(mod_key).unwrap_or(&0)));
        }
        self.should_measure(mod_key)
            .then(|| collect_directory_size_bytes(path))
            .transpose()
    }
}

fn path_key_is_ancestor_or_descendant(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
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

/// A complete onboarding discovery and work plan derived from one metadata
/// walk. `directory_sizes` feeds terminal mod sizes during strict discovery.
#[derive(Debug, Clone)]
pub struct OnboardingDiskDiscovery {
    pub discovery: DiskScopedDiscovery,
    pub work_plan: crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingWorkPlan,
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
    pub files_inspected: u64,
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

pub(crate) fn filesystem_identity(path: &Path) -> Option<String> {
    let id = file_id::get_file_id(path).ok()?;
    Some(match id {
        file_id::FileId::Inode {
            device_id,
            inode_number,
        } => format!("inode:{device_id}:{inode_number}"),
        file_id::FileId::LowRes {
            volume_serial_number,
            file_index,
        } => format!("win-low:{volume_serial_number}:{file_index}"),
        file_id::FileId::HighRes {
            volume_serial_number,
            file_id,
        } => format!("win-high:{volume_serial_number}:{file_id}"),
    })
}

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
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }

    let top_level_paths = list_runtime_dirs(mods_path)?;
    let mut pending = top_level_paths.clone();
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
    entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });

    Ok(DiskIdentityCensus {
        entries,
        top_level_roots: top_level_paths.len(),
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
    entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });

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
    size_scan: Option<&DiskSizeScan>,
) -> DiskProjectionResult<()> {
    classified_directories.fetch_add(1, Ordering::Relaxed);
    let (node_type, _reasons, _warnings) = classify_folder_strict(path)
        .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
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
            let size_bytes = size_scan
                .map(|scan| scan.size_for(&folder_path_key, path))
                .transpose()?
                .flatten();
            mods.push(DiskModEntry {
                folder_path: folder_path.clone(),
                folder_path_key,
                object_folder_path_key: object_folder_path_key.to_string(),
                raw_name,
                absolute_path: path.to_path_buf(),
                filesystem_identity: filesystem_identity(path),
                size_bytes,
            });
            Ok(())
        }
        NodeType::InternalAssets => Ok(()),
        NodeType::ContainerFolder => {
            for child_path in list_runtime_dirs(path)? {
                collect_terminal_mods(
                    mods,
                    mods_path,
                    object_folder_path_key,
                    &child_path,
                    classified_directories,
                    size_scan,
                )?;
            }
            Ok(())
        }
    }
}

fn collect_directory_size_bytes(path: &Path) -> DiskProjectionResult<i64> {
    let mut total_bytes = 0_u64;
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry.map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to read file metadata while sizing '{}': {error}",
                path.display()
            ))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to read file metadata for '{}': {error}",
                entry.path().display()
            ))
        })?;
        total_bytes = total_bytes.checked_add(metadata.len()).ok_or_else(|| {
            DiskProjectionError::Failed(format!(
                "Storage size overflow while sizing '{}'",
                path.display()
            ))
        })?;
    }
    i64::try_from(total_bytes).map_err(|_| {
        DiskProjectionError::Failed(format!(
            "Storage size exceeds supported range for '{}'",
            path.display()
        ))
    })
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
    collect_disk_projection_with_progress_and_stats(
        mods_path,
        changed_roots,
        scoped,
        progress,
        None,
    )
    .map(|(projection, _)| projection)
}

fn collect_disk_projection_with_progress_and_stats(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
    size_scan: Option<&DiskSizeScan>,
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

            let mut mods = Vec::new();
            for mod_path in list_runtime_dirs(root_path)? {
                collect_terminal_mods(
                    &mut mods,
                    mods_path,
                    &object_entry.folder_path_key,
                    &mod_path,
                    &classified_directories,
                    size_scan,
                )?;
            }

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
    size_scan: Option<&DiskSizeScan>,
    progress: Option<&(dyn Fn(DiskSnapshotProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskScopedDiscovery> {
    collect_scoped_disk_discovery_with_census(
        mods_path,
        changed_roots,
        scoped,
        size_scan,
        None,
        progress,
    )
}

/// Trusted internal mutations have already validated every source,
/// destination, sibling collision, and filesystem identity under the game
/// mutation lease. Their terminal projection can therefore avoid observing
/// unrelated roots entirely.
pub fn collect_trusted_scoped_disk_discovery_with_progress(
    mods_path: &Path,
    changed_roots: &[String],
    size_scan: Option<&DiskSizeScan>,
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
    let (projection, classified_directories) = collect_disk_projection_with_progress_and_stats(
        mods_path,
        changed_roots,
        true,
        progress,
        size_scan,
    )?;
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
    size_scan: Option<&DiskSizeScan>,
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
        size_scan,
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

/// Build the onboarding work plan and the terminal-mod size index with one
/// metadata walk. Strict classification still reads folders and INI files, but
/// it never performs a second recursive metadata walk for size accounting.
pub fn collect_onboarding_disk_discovery(
    game_id: String,
    mods_path: &Path,
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    collect_onboarding_disk_discovery_with_progress(game_id, mods_path, None)
}

pub fn collect_onboarding_disk_discovery_with_progress(
    game_id: String,
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    let metadata = collect_onboarding_metadata(mods_path, progress)?;
    let roots = metadata.roots;
    let file_count = roots.iter().map(|root| root.file_count).sum();
    let total_bytes = roots.iter().map(|root| root.total_bytes).sum();
    let work_units = roots.iter().map(|root| root.work_units).sum();
    let work_plan = crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingWorkPlan {
        game_id,
        file_count,
        total_bytes,
        work_units,
        roots,
    };
    let size_scan = DiskSizeScan::precomputed(metadata.directory_sizes);
    let classification_progress = |snapshot: DiskSnapshotProgress| {
        if let Some(progress) = progress {
            progress(OnboardingDiscoveryProgress {
                phase: OnboardingDiscoveryPhase::Classifying,
                completed_roots: snapshot.completed_roots,
                total_roots: snapshot.total_roots,
                files_inspected: file_count,
                folders_classified: snapshot.classified_directories,
                current_root: snapshot.current_root,
                is_terminal: snapshot.total_roots == 0
                    || snapshot.completed_roots == snapshot.total_roots,
            });
        }
    };
    let discovery = collect_scoped_disk_discovery_with_census(
        mods_path,
        &[],
        false,
        Some(&size_scan),
        Some(metadata.census),
        progress.map(|_| &classification_progress as _),
    )?;
    Ok(OnboardingDiskDiscovery {
        discovery,
        work_plan,
    })
}

/// Rebuild only the changed onboarding roots. The global directory census is
/// still collected by scoped discovery to retain conflict correctness.
pub fn collect_scoped_onboarding_disk_discovery(
    game_id: String,
    mods_path: &Path,
    changed_roots: &[String],
) -> DiskProjectionResult<OnboardingDiskDiscovery> {
    let (roots, directory_sizes) = collect_onboarding_metadata_for_roots(mods_path, changed_roots)?;
    let file_count = roots.iter().map(|root| root.file_count).sum();
    let total_bytes = roots.iter().map(|root| root.total_bytes).sum();
    let work_units = roots.iter().map(|root| root.work_units).sum();
    let work_plan = crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingWorkPlan {
        game_id,
        file_count,
        total_bytes,
        work_units,
        roots,
    };
    let size_scan = DiskSizeScan::precomputed(directory_sizes);
    let discovery = collect_scoped_disk_discovery_with_progress(
        mods_path,
        changed_roots,
        true,
        Some(&size_scan),
        None,
    )?;
    Ok(OnboardingDiskDiscovery {
        discovery,
        work_plan,
    })
}

fn collect_onboarding_metadata_for_roots(
    mods_path: &Path,
    root_names: &[String],
) -> DiskProjectionResult<(
    Vec<crate::modules::reconciliation::application::disk_reconcile::types::IndexingRootWork>,
    HashMap<String, i64>,
)> {
    let mut roots = BTreeMap::<String, (u64, u64)>::new();
    let mut directory_sizes = HashMap::<String, u64>::new();
    for root_name in root_names.iter().collect::<BTreeSet<_>>() {
        let root_path = mods_path.join(root_name);
        if !root_path.is_dir() {
            continue;
        }
        roots.entry((*root_name).to_string()).or_default();
        for entry in WalkDir::new(&root_path).follow_links(false) {
            let entry = entry.map_err(|error| {
                DiskProjectionError::Failed(format!(
                    "Failed to read file metadata while planning '{}': {error}",
                    root_path.display()
                ))
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let metadata = entry.metadata().map_err(|error| {
                DiskProjectionError::Failed(format!(
                    "Failed to read file metadata for '{}': {error}",
                    entry.path().display()
                ))
            })?;
            let root = roots.entry((*root_name).to_string()).or_default();
            root.0 = root.0.saturating_add(1);
            root.1 = root.1.saturating_add(metadata.len());
            let mut directory = entry.path().parent();
            while let Some(path) = directory {
                if !path.starts_with(&root_path) {
                    break;
                }
                let relative_directory = path.strip_prefix(mods_path).map_err(|error| {
                    DiskProjectionError::Failed(format!(
                        "Failed to compute directory size key for '{}': {error}",
                        path.display()
                    ))
                })?;
                let key = crate::shared::path_key::folder_path_key(
                    &relative_directory.to_string_lossy(),
                    None,
                );
                let total = directory_sizes.entry(key).or_default();
                *total = total.checked_add(metadata.len()).ok_or_else(|| {
                    DiskProjectionError::Failed(format!(
                        "Storage size overflow while sizing '{}'",
                        root_path.display()
                    ))
                })?;
                directory = path.parent();
            }
        }
    }
    let roots = roots.into_iter().map(|(name, (files, bytes))| {
        crate::modules::reconciliation::application::disk_reconcile::work_plan::indexing_root_work(name, files, bytes)
    }).collect();
    let directory_sizes = directory_sizes
        .into_iter()
        .map(|(key, bytes)| {
            i64::try_from(bytes).map(|bytes| (key, bytes)).map_err(|_| {
                DiskProjectionError::Failed(format!(
                    "Storage size exceeds supported range for '{}'",
                    mods_path.display()
                ))
            })
        })
        .collect::<DiskProjectionResult<HashMap<_, _>>>()?;
    Ok((roots, directory_sizes))
}

struct OnboardingMetadata {
    roots:
        Vec<crate::modules::reconciliation::application::disk_reconcile::types::IndexingRootWork>,
    directory_sizes: HashMap<String, i64>,
    census: DiskIdentityCensus,
}

fn collect_onboarding_metadata(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
) -> DiskProjectionResult<OnboardingMetadata> {
    if !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }

    let top_level_paths = list_runtime_dirs(mods_path)?;
    let mut roots = BTreeMap::<String, (u64, u64)>::new();
    for root in &top_level_paths {
        roots.entry(runtime_dir_name(root)).or_default();
    }
    let runtime_root_names = roots.keys().cloned().collect::<BTreeSet<_>>();
    let total_roots = runtime_root_names.len();
    let mut census_entries = Vec::new();
    let mut directory_paths = BTreeSet::from([mods_path.to_path_buf()]);
    let mut direct_directory_sizes = HashMap::<PathBuf, u64>::new();
    let mut files_inspected = 0_u64;
    let mut completed_roots = 0_usize;
    let mut active_root = None::<String>;

    if let Some(progress) = progress {
        progress(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots,
            total_roots,
            files_inspected,
            folders_classified: 0,
            current_root: None,
            is_terminal: false,
        });
    }

    for entry in WalkDir::new(mods_path).follow_links(false) {
        let entry = entry.map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to read file metadata while planning '{}': {error}",
                mods_path.display()
            ))
        })?;
        let path = entry.path();
        let relative = path.strip_prefix(mods_path).map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to compute relative file path for '{}': {error}",
                path.display()
            ))
        })?;

        let next_root = relative
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .filter(|root| runtime_root_names.contains(*root))
            .map(str::to_string);
        if next_root != active_root {
            if active_root.is_some() {
                completed_roots = completed_roots.saturating_add(1).min(total_roots);
            }
            active_root = next_root;
        }

        if entry.file_type().is_dir() {
            directory_paths.insert(path.to_path_buf());
            if is_runtime_census_directory(relative) {
                let folder_path = relative.to_string_lossy().to_string();
                census_entries.push(DiskIdentityCensusEntry {
                    folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
                    folder_path,
                    raw_name: runtime_dir_name(path),
                    absolute_path: path.to_path_buf(),
                });
            }
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to read file metadata for '{}': {error}",
                entry.path().display()
            ))
        })?;
        let root_name = if relative.components().count() == 1 {
            ".".to_string()
        } else {
            relative
                .components()
                .next()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| ".".to_string())
        };
        let root = roots.entry(root_name).or_default();
        root.0 = root.0.saturating_add(1);
        root.1 = root.1.saturating_add(metadata.len());
        let parent = path.parent().ok_or_else(|| {
            DiskProjectionError::Failed(format!(
                "File '{}' has no parent directory",
                path.display()
            ))
        })?;
        let total = direct_directory_sizes
            .entry(parent.to_path_buf())
            .or_default();
        *total = total.checked_add(metadata.len()).ok_or_else(|| {
            DiskProjectionError::Failed(format!(
                "Storage size overflow while sizing '{}'",
                mods_path.display()
            ))
        })?;
        files_inspected = files_inspected.saturating_add(1);
        if files_inspected.is_multiple_of(128) {
            if let Some(progress) = progress {
                progress(OnboardingDiscoveryProgress {
                    phase: OnboardingDiscoveryPhase::Metadata,
                    completed_roots,
                    total_roots,
                    files_inspected,
                    folders_classified: 0,
                    current_root: active_root.clone(),
                    is_terminal: false,
                });
            }
        }
    }

    if active_root.is_some() {
        completed_roots = completed_roots.saturating_add(1).min(total_roots);
    }
    census_entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });
    let directory_sizes =
        roll_up_directory_sizes(mods_path, directory_paths, direct_directory_sizes)?;

    let roots = roots
        .into_iter()
        .map(|(root_name, (file_count, total_bytes))| {
            crate::modules::reconciliation::application::disk_reconcile::work_plan::indexing_root_work(
                root_name,
                file_count,
                total_bytes,
            )
        })
        .collect();
    if let Some(progress) = progress {
        progress(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots,
            total_roots,
            files_inspected,
            folders_classified: 0,
            current_root: active_root,
            is_terminal: true,
        });
    }
    Ok(OnboardingMetadata {
        roots,
        directory_sizes,
        census: DiskIdentityCensus {
            entries: census_entries,
            top_level_roots: total_roots,
        },
    })
}

fn is_runtime_census_directory(relative: &Path) -> bool {
    !relative.as_os_str().is_empty()
        && relative.components().all(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| !name.starts_with('.'))
        })
}

fn roll_up_directory_sizes(
    mods_path: &Path,
    directory_paths: BTreeSet<PathBuf>,
    mut directory_sizes: HashMap<PathBuf, u64>,
) -> DiskProjectionResult<HashMap<String, i64>> {
    let mut directories = directory_paths.into_iter().collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    for directory in &directories {
        let size = directory_sizes.get(directory).copied().unwrap_or_default();
        if size == 0 || directory == mods_path {
            continue;
        }
        let Some(parent) = directory
            .parent()
            .filter(|path| path.starts_with(mods_path))
        else {
            continue;
        };
        let total = directory_sizes.entry(parent.to_path_buf()).or_default();
        *total = total.checked_add(size).ok_or_else(|| {
            DiskProjectionError::Failed(format!(
                "Storage size overflow while sizing '{}'",
                mods_path.display()
            ))
        })?;
    }

    let mut normalized_sizes = HashMap::<String, u64>::new();
    for (directory, size) in directory_sizes {
        if size == 0 || !directory.starts_with(mods_path) {
            continue;
        }
        let relative = directory.strip_prefix(mods_path).map_err(|error| {
            DiskProjectionError::Failed(format!(
                "Failed to compute directory size key for '{}': {error}",
                directory.display()
            ))
        })?;
        let key = crate::shared::path_key::folder_path_key(&relative.to_string_lossy(), None);
        let total = normalized_sizes.entry(key).or_default();
        *total = total.checked_add(size).ok_or_else(|| {
            DiskProjectionError::Failed(format!(
                "Storage size overflow while sizing '{}'",
                mods_path.display()
            ))
        })?;
    }

    normalized_sizes
        .into_iter()
        .map(|(key, bytes)| {
            i64::try_from(bytes).map(|bytes| (key, bytes)).map_err(|_| {
                DiskProjectionError::Failed(format!(
                    "Storage size exceeds supported range for '{}'",
                    mods_path.display()
                ))
            })
        })
        .collect()
}

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
    fn onboarding_discovery_reuses_one_metadata_walk_for_plan_and_mod_sizes() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let terminal = temp.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(temp.path().join("d3dx.ini"), b"abc")
            .expect("direct root file should be written");
        let ini_bytes = b"[TextureOverrideTest]\nhash = abc\n";
        std::fs::write(terminal.join("mod.ini"), ini_bytes).expect("mod ini should be written");
        std::fs::write(terminal.join("mesh.buf"), b"mesh").expect("mod asset should be written");

        let onboarding = collect_onboarding_disk_discovery("game".to_string(), temp.path())
            .expect("onboarding discovery should succeed");

        assert_eq!(onboarding.work_plan.file_count, 3);
        assert_eq!(
            onboarding.work_plan.total_bytes,
            3 + ini_bytes.len() as u64 + 4
        );
        assert!(onboarding
            .work_plan
            .roots
            .iter()
            .any(|root| root.root_name == "." && root.file_count == 1));
        assert_eq!(
            onboarding.discovery.projection.mods[0].size_bytes,
            Some((ini_bytes.len() + 4) as i64)
        );
    }

    #[test]
    fn onboarding_metadata_preserves_direct_files_hidden_folders_and_nested_mod_sizes() {
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

        let metadata = collect_onboarding_metadata(temp.path(), None)
            .expect("metadata collection should succeed");

        assert_eq!(
            metadata
                .roots
                .iter()
                .map(|root| root.file_count)
                .sum::<u64>(),
            4
        );
        assert!(metadata.roots.iter().any(|root| root.root_name == "."));
        assert!(metadata.roots.iter().any(|root| root.root_name == ".cache"));
        assert!(metadata
            .census
            .entries
            .iter()
            .all(|entry| !entry.folder_path.starts_with(".cache")));

        let nested_key = crate::shared::path_key::folder_path_key("Alice/Nested/Blue Dress", None);
        assert_eq!(
            metadata.directory_sizes.get(&nested_key),
            Some(&((ini_bytes.len() + 5) as i64))
        );

        let onboarding = collect_onboarding_disk_discovery("game".to_string(), temp.path())
            .expect("onboarding discovery should succeed");
        assert_eq!(onboarding.discovery.projection.mods.len(), 1);
        assert_eq!(
            onboarding.discovery.projection.mods[0].size_bytes,
            Some((ini_bytes.len() + 5) as i64)
        );
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
    fn scoped_onboarding_discovery_measures_only_changed_roots() {
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

        let onboarding = collect_scoped_onboarding_disk_discovery(
            "game".to_string(),
            temp.path(),
            &["Bob".to_string()],
        )
        .expect("scoped onboarding discovery should succeed");

        assert!(onboarding.discovery.scoped);
        assert_eq!(onboarding.work_plan.roots.len(), 1);
        assert_eq!(onboarding.work_plan.roots[0].root_name, "Bob");
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
    fn storage_scan_measures_only_changed_or_new_terminal_mods() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let blue = temp.path().join("Alice").join("Blue");
        let red = temp.path().join("Alice").join("Red");
        std::fs::create_dir_all(&blue).expect("blue folder");
        std::fs::create_dir_all(&red).expect("red folder");
        std::fs::write(blue.join("mod.ini"), "[TextureOverrideBlue]\nhash = abc\n")
            .expect("blue ini");
        std::fs::write(red.join("mod.ini"), "[TextureOverrideRed]\nhash = def\n").expect("red ini");
        std::fs::write(red.join("mesh.buf"), [1_u8; 8]).expect("red asset");
        std::fs::write(temp.path().join("Alice").join("outside.bin"), [2_u8; 64])
            .expect("object-only asset");

        let changed_file = red.join("mesh.buf").to_string_lossy().to_string();
        let known_mod_keys = ["Alice/Blue", "Alice/Red"]
            .into_iter()
            .map(|path| crate::shared::path_key::folder_path_key(path, None))
            .collect();
        let size_scan = DiskSizeScan::incremental(temp.path(), known_mod_keys, &[changed_file]);
        let discovery = collect_scoped_disk_discovery_with_progress(
            temp.path(),
            &[],
            false,
            Some(&size_scan),
            None,
        )
        .expect("snapshot should succeed");

        let blue = discovery
            .projection
            .mods
            .iter()
            .find(|entry| entry.raw_name == "Blue")
            .expect("blue mod");
        let red = discovery
            .projection
            .mods
            .iter()
            .find(|entry| entry.raw_name == "Red")
            .expect("red mod");
        assert_eq!(blue.size_bytes, None);
        assert_eq!(red.size_bytes, Some(40));
    }

    #[test]
    fn storage_scan_deduplicates_normalized_changed_paths() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let changed_path = temp.path().join("Alice").join("Blue").join("mod.ini");
        let changed_path = changed_path.to_string_lossy().into_owned();
        let size_scan = DiskSizeScan::incremental(
            temp.path(),
            HashSet::new(),
            &[changed_path.clone(), changed_path],
        );

        assert_eq!(size_scan.changed_path_keys.len(), 1);
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
                collect_scoped_disk_discovery_with_progress(temp.path(), &[], false, None, None)
                    .expect("full snapshot should succeed");
            let cold_full_elapsed = started.elapsed();
            let started = std::time::Instant::now();
            let cold_scoped = collect_trusted_scoped_disk_discovery_with_progress(
                temp.path(),
                &["Object 000".to_string()],
                None,
                None,
            )
            .expect("trusted scoped snapshot should succeed");
            let cold_scoped_elapsed = started.elapsed();

            let mut warm_full = Vec::new();
            let mut warm_scoped = Vec::new();
            for _ in 0..5 {
                let started = std::time::Instant::now();
                collect_scoped_disk_discovery_with_progress(temp.path(), &[], false, None, None)
                    .expect("warm full snapshot should succeed");
                warm_full.push(started.elapsed());
                let started = std::time::Instant::now();
                collect_trusted_scoped_disk_discovery_with_progress(
                    temp.path(),
                    &["Object 000".to_string()],
                    None,
                    None,
                )
                .expect("warm trusted scoped snapshot should succeed");
                warm_scoped.push(started.elapsed());
            }

            assert_eq!(cold_full.projection.mods.len(), total_mods);
            assert_eq!(cold_full.scan_counts.classified_directories, total_mods);
            assert_eq!(
                cold_scoped.scan_counts.classified_directories,
                total_mods.min(mods_per_object)
            );
            assert!(cold_scoped.scoped);
            let full_p50 = percentile(&mut warm_full.clone(), 50);
            let full_p95 = percentile(&mut warm_full, 95);
            let scoped_p50 = percentile(&mut warm_scoped.clone(), 50);
            let scoped_p95 = percentile(&mut warm_scoped, 95);
            eprintln!(
                "disk_snapshot_{total_mods}: first_full={cold_full_elapsed:?} first_scoped={cold_scoped_elapsed:?} warm_full_p50={full_p50:?} warm_full_p95={full_p95:?} warm_scoped_p50={scoped_p50:?} warm_scoped_p95={scoped_p95:?} scoped_census={} scoped_classified={}",
                cold_scoped.scan_counts.census_directories,
                cold_scoped.scan_counts.classified_directories,
            );
        }
    }
}
