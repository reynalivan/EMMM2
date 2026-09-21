use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::modules::workspace::domain::classifier::{
    classify_folder_strict, classify_folder_strict_from_scan, scan_folder_strict, NodeType,
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
    /// `None` when an incremental reconcile did not need to rescan this mod.
    pub size_bytes: Option<i64>,
}

/// Restricts size metadata walks to new or changed terminal mods so ordinary
/// reconciliation never scans an entire library for dashboard statistics.
#[derive(Debug, Clone, Default)]
pub struct DiskSizeScan {
    scan_all: bool,
    known_mod_keys: HashSet<String>,
    changed_path_keys: Vec<String>,
}

impl DiskSizeScan {
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
    collect_disk_identity_census_with_progress(mods_path, None)
}

fn collect_disk_identity_census_with_progress(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
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

    let mut entries = Vec::new();
    for (index, root_path) in top_level_paths.iter().enumerate() {
        let root_name = runtime_dir_name(root_path);
        let mut pending = vec![root_path.clone()];
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
        if let Some(progress) = progress {
            progress(OnboardingDiscoveryProgress {
                phase: OnboardingDiscoveryPhase::Metadata,
                completed_roots: index + 1,
                total_roots,
                folders_classified: 0,
                current_root: Some(root_name),
                is_terminal: index + 1 == total_roots,
            });
        }
    }
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
    entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });

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
    let roots = top_level_paths
        .par_iter()
        .map(|root_path| {
            let root =
                collect_onboarding_root_discovery(mods_path, root_path, &classified_directories)?;
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

    let mut census_entries = Vec::new();
    let mut projection = DiskProjection::default();
    let mut classification_duration = Duration::ZERO;
    for root in roots {
        census_duration += root.census_duration;
        classification_duration += root.classification_duration;
        census_entries.extend(root.census_entries);
        projection.objects.push(root.object);
        projection.mods.extend(root.mods);
    }
    census_entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });
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
    census_entries: Vec<DiskIdentityCensusEntry>,
    object: DiskObjectEntry,
    mods: Vec<DiskModEntry>,
    census_duration: Duration,
    classification_duration: Duration,
}

fn collect_onboarding_root_discovery(
    mods_path: &Path,
    root_path: &Path,
    classified_directories: &AtomicUsize,
) -> DiskProjectionResult<OnboardingRootDiscovery> {
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
        census_entries: vec![census_entry(mods_path, root_path)?],
        object,
        mods: Vec::new(),
        census_duration: Duration::ZERO,
        classification_duration: Duration::ZERO,
    };
    let object_folder_path_key = result.object.folder_path_key.clone();
    let root_children = timed_runtime_dirs(root_path, &mut result.census_duration)?;
    for child_path in root_children {
        collect_onboarding_folder(
            &child_path,
            mods_path,
            &object_folder_path_key,
            &mut result,
            classified_directories,
        )?;
    }
    Ok(result)
}

fn collect_onboarding_folder(
    path: &Path,
    mods_path: &Path,
    object_folder_path_key: &str,
    result: &mut OnboardingRootDiscovery,
    classified_directories: &AtomicUsize,
) -> DiskProjectionResult<()> {
    result.census_entries.push(census_entry(mods_path, path)?);
    classified_directories.fetch_add(1, Ordering::Relaxed);
    let scan_started = std::time::Instant::now();
    let scan =
        scan_folder_strict(path).map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
    result.census_duration += scan_started.elapsed();
    let classification_started = std::time::Instant::now();
    let (node_type, _reasons, _warnings) = classify_folder_strict_from_scan(&scan)
        .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
    result.classification_duration += classification_started.elapsed();

    let children = scan.child_dirs;
    match node_type {
        NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
            result.mods.push(onboarding_mod_entry(
                mods_path,
                object_folder_path_key,
                path,
            )?);
            for child_path in children {
                collect_onboarding_census_only(&child_path, mods_path, result)?;
            }
        }
        NodeType::InternalAssets => {
            for child_path in children {
                collect_onboarding_census_only(&child_path, mods_path, result)?;
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
                )?;
            }
        }
    }
    Ok(())
}

fn collect_onboarding_census_only(
    path: &Path,
    mods_path: &Path,
    result: &mut OnboardingRootDiscovery,
) -> DiskProjectionResult<()> {
    result.census_entries.push(census_entry(mods_path, path)?);
    for child_path in timed_runtime_dirs(path, &mut result.census_duration)? {
        collect_onboarding_census_only(&child_path, mods_path, result)?;
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
        size_bytes: None,
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
        collect_scoped_disk_discovery_with_progress(mods_path, changed_roots, true, None, None)?;
    Ok(OnboardingDiskDiscovery { discovery })
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
    fn onboarding_discovery_skips_file_size_measurement() {
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

        assert_eq!(onboarding.discovery.projection.mods[0].size_bytes, None);
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
        assert_eq!(onboarding.discovery.projection.mods[0].size_bytes, None);
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
            collect_disk_projection_with_progress_and_stats(temp.path(), &[], false, None, None)
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
                        entry.size_bytes,
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
            let cold_onboarding = collect_onboarding_disk_discovery(temp.path())
                .expect("onboarding discovery should succeed");
            let cold_onboarding_elapsed = started.elapsed();
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
            let mut warm_onboarding = Vec::new();
            let mut warm_scoped = Vec::new();
            for _ in 0..5 {
                let started = std::time::Instant::now();
                collect_scoped_disk_discovery_with_progress(temp.path(), &[], false, None, None)
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
