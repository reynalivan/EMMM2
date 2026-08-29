use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::common::classifier::{classify_folder_strict, NodeType};
use crate::common::normalizer::{is_disabled_folder, normalize_display_name};

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

#[derive(Debug, Clone)]
pub struct DiskScopedDiscovery {
    pub census: DiskIdentityCensus,
    pub projection: DiskProjection,
    pub scoped: bool,
    pub scan_counts: DiskDiscoveryScanCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskSnapshotProgress {
    pub completed_roots: usize,
    pub total_roots: usize,
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
            .map(|root| crate::common::path_key::folder_path_key(root, None))
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
            folder_path_key: crate::common::path_key::folder_path_key(&folder_path, None),
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

fn collect_terminal_mods(
    mods: &mut Vec<DiskModEntry>,
    mods_path: &Path,
    object_folder_path_key: &str,
    path: &Path,
    classified_directories: &AtomicUsize,
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
            mods.push(DiskModEntry {
                folder_path: folder_path.clone(),
                folder_path_key: crate::common::path_key::folder_path_key(&folder_path, None),
                object_folder_path_key: object_folder_path_key.to_string(),
                raw_name,
                absolute_path: path.to_path_buf(),
                filesystem_identity: filesystem_identity(path),
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
                        current_root: Some(root_name.clone()),
                    });
                }
                return Ok(None);
            }

            let object_entry = DiskObjectEntry {
                folder_path: root_name.clone(),
                folder_path_key: crate::common::path_key::folder_path_key(root_name, None),
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
                )?;
            }

            let entry = Ok(Some((object_entry, mods)));
            let completed = completed_roots.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(progress) = progress {
                progress(DiskSnapshotProgress {
                    completed_roots: completed,
                    total_roots,
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
    let census = collect_disk_identity_census(mods_path)?;
    let scoped = scoped && !census.has_cross_root_ambiguity();
    let (projection, classified_directories) = collect_disk_projection_with_progress_and_stats(
        mods_path,
        changed_roots,
        scoped,
        progress,
    )?;
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
        census,
        projection,
        scoped,
    })
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
    #[ignore = "manual 10k-folder performance fixture"]
    fn benchmark_disk_snapshot_10k_folders() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        const OBJECTS: usize = 100;
        const MODS_PER_OBJECT: usize = 100;
        for object_index in 0..OBJECTS {
            for mod_index in 0..MODS_PER_OBJECT {
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
        let full = collect_scoped_disk_discovery_with_progress(temp.path(), &[], false, None)
            .expect("full snapshot should succeed");
        let full_elapsed = started.elapsed();
        let started = std::time::Instant::now();
        let scoped = collect_scoped_disk_discovery_with_progress(
            temp.path(),
            &["Object 000".to_string()],
            true,
            None,
        )
        .expect("scoped snapshot should succeed");
        let scoped_elapsed = started.elapsed();

        assert_eq!(full.projection.objects.len(), OBJECTS);
        assert_eq!(full.projection.mods.len(), OBJECTS * MODS_PER_OBJECT);
        assert_eq!(
            full.scan_counts.classified_directories,
            OBJECTS * MODS_PER_OBJECT
        );
        assert_eq!(scoped.scan_counts.classified_directories, MODS_PER_OBJECT);
        eprintln!(
            "disk_snapshot_10k: full={} strict directories in {full_elapsed:?}; scoped={} strict directories in {scoped_elapsed:?}; census={} directories",
            full.scan_counts.classified_directories,
            scoped.scan_counts.classified_directories,
            scoped.scan_counts.census_directories,
        );
    }
}
