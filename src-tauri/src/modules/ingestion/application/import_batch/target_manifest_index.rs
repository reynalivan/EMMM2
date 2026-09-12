//! Per-batch cache for installed payload manifests.
//!
//! The cache never survives an application restart and each lookup rebuilds a
//! metadata snapshot. That keeps repeat review operations fast without using a
//! cached manifest as commit-time proof.

use super::payload_manifest::{
    build_payload_manifest, compare_payload_manifests, ManifestComparisonKind, PayloadManifest,
    PayloadManifestMetadata,
};
use crate::shared::errors::AppError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[derive(Clone, Default)]
pub struct TargetManifestIndexState {
    entries: Arc<Mutex<BTreeMap<BatchRootKey, BTreeMap<TargetRootKey, CachedTarget>>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BatchRootKey {
    batch_id: String,
    game_id: String,
    mods_root_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TargetRootKey {
    path_key: String,
    active_identity: String,
}

#[derive(Clone)]
struct CachedTarget {
    path: PathBuf,
    snapshot: TargetSnapshot,
    metadata: PayloadManifestMetadata,
    manifest: Option<PayloadManifest>,
}

#[derive(Clone, PartialEq, Eq)]
struct TargetSnapshot(Vec<TargetSnapshotEntry>);

#[derive(Clone, PartialEq, Eq)]
struct TargetSnapshotEntry {
    relative_path: String,
    size_bytes: u64,
    modified_unix_ms: u128,
}

#[derive(Clone)]
struct ObservedTarget {
    key: TargetRootKey,
    path: PathBuf,
    snapshot: TargetSnapshot,
    metadata: PayloadManifestMetadata,
}

impl TargetManifestIndexState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_batch(&self, batch_id: &str) {
        let mut entries = crate::shared::sync::lock(&self.entries);
        entries.retain(|key, _| key.batch_id != batch_id);
    }

    /// Finds a byte-identical installed payload while reusing a manifest only
    /// when the root's full metadata snapshot is unchanged.
    pub fn find_existing_payload_match(
        &self,
        batch_id: &str,
        game_id: &str,
        mods_root: &Path,
        source: &PayloadManifest,
    ) -> Result<Option<PathBuf>, AppError> {
        let key = BatchRootKey {
            batch_id: batch_id.to_string(),
            game_id: game_id.to_string(),
            mods_root_key: crate::shared::path_key::canonical_path_key_for_path(mods_root),
        };
        let observed = observe_mod_roots(mods_root)?;
        self.refresh_root_snapshot(&key, &observed);

        for target in observed {
            if target.metadata.file_count != source.file_count
                || target.metadata.total_size_bytes != source.total_size_bytes
            {
                continue;
            }
            let manifest = self.manifest_for(&key, &target)?;
            if compare_payload_manifests(source, &manifest).kind == ManifestComparisonKind::Exact {
                return Ok(Some(target.path));
            }
        }
        Ok(None)
    }

    pub fn manifest_for_path(
        &self,
        batch_id: &str,
        game_id: &str,
        mods_root: &Path,
        target_path: &Path,
    ) -> Result<PayloadManifest, AppError> {
        let key = BatchRootKey {
            batch_id: batch_id.to_string(),
            game_id: game_id.to_string(),
            mods_root_key: crate::shared::path_key::canonical_path_key_for_path(mods_root),
        };
        let observed = observe_mod_roots(mods_root)?;
        self.refresh_root_snapshot(&key, &observed);
        let target_key = target_root_key(target_path);
        let target = observed
            .into_iter()
            .find(|candidate| candidate.key == target_key)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "Existing target is no longer a mod root: {}",
                    target_path.display()
                ))
            })?;
        self.manifest_for(&key, &target)
    }

    fn refresh_root_snapshot(&self, key: &BatchRootKey, observed: &[ObservedTarget]) {
        let observed_keys = observed
            .iter()
            .map(|target| target.key.clone())
            .collect::<BTreeSet<_>>();
        let mut entries = crate::shared::sync::lock(&self.entries);
        entries.retain(|existing, _| {
            existing.batch_id != key.batch_id
                || existing.game_id != key.game_id
                || existing.mods_root_key == key.mods_root_key
        });
        let roots = entries.entry(key.clone()).or_default();
        roots.retain(|target_key, _| observed_keys.contains(target_key));
        for target in observed {
            match roots.get_mut(&target.key) {
                Some(cached) if cached.snapshot == target.snapshot => {
                    cached.path = target.path.clone();
                    cached.metadata = target.metadata.clone();
                }
                Some(cached) => {
                    cached.path = target.path.clone();
                    cached.snapshot = target.snapshot.clone();
                    cached.metadata = target.metadata.clone();
                    cached.manifest = None;
                }
                None => {
                    roots.insert(
                        target.key.clone(),
                        CachedTarget {
                            path: target.path.clone(),
                            snapshot: target.snapshot.clone(),
                            metadata: target.metadata.clone(),
                            manifest: None,
                        },
                    );
                }
            }
        }
    }

    fn manifest_for(
        &self,
        key: &BatchRootKey,
        target: &ObservedTarget,
    ) -> Result<PayloadManifest, AppError> {
        if let Some(manifest) = crate::shared::sync::lock(&self.entries)
            .get(key)
            .and_then(|roots| roots.get(&target.key))
            .and_then(|cached| cached.manifest.clone())
        {
            return Ok(manifest);
        }
        let manifest = build_payload_manifest(&target.path, None)?;
        let mut entries = crate::shared::sync::lock(&self.entries);
        let Some(cached) = entries
            .get_mut(key)
            .and_then(|roots| roots.get_mut(&target.key))
        else {
            return Err(AppError::Validation(
                "Target manifest cache changed during inspection; retry the review".to_string(),
            ));
        };
        if cached.snapshot != target.snapshot {
            return Err(AppError::Validation(
                "Target changed during inspection; retry the review".to_string(),
            ));
        }
        cached.manifest = Some(manifest.clone());
        Ok(manifest)
    }
}

fn observe_mod_roots(mods_root: &Path) -> Result<Vec<ObservedTarget>, AppError> {
    if !mods_root.is_dir() {
        return Err(AppError::Validation(format!(
            "Configured mods path is not a folder: {}",
            mods_root.display()
        )));
    }
    let roots =
        crate::modules::library::application::mods::archive::classify::find_mod_roots_with_limits(
            mods_root,
            crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_DEPTH,
            crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_ENTRIES,
            None,
        )?
        .roots;
    roots
        .into_iter()
        .map(|path| {
            let (snapshot, metadata) = snapshot_target(&path)?;
            Ok(ObservedTarget {
                key: target_root_key(&path),
                path,
                snapshot,
                metadata,
            })
        })
        .collect()
}

fn target_root_key(path: &Path) -> TargetRootKey {
    let folder_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let active_name = folder_name
        .strip_prefix("DISABLED ")
        .or_else(|| folder_name.strip_prefix("disabled "))
        .unwrap_or(&folder_name);
    TargetRootKey {
        path_key: crate::shared::path_key::canonical_path_key_for_path(path),
        active_identity: crate::modules::workspace::domain::normalizer::normalize_display_name(
            active_name,
        )
        .to_string(),
    }
}

fn snapshot_target(root: &Path) -> Result<(TargetSnapshot, PayloadManifestMetadata), AppError> {
    let canonical = root.canonicalize().map_err(|error| {
        AppError::Validation(format!(
            "Target metadata root '{}' is unavailable: {error}",
            root.display()
        ))
    })?;
    let mut entries = Vec::new();
    let mut file_count = 0_u32;
    let mut total_size_bytes = 0_u64;
    for entry in WalkDir::new(&canonical)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "Could not read target metadata '{}': {error}",
                canonical.display()
            ))
        })?;
        if entry.path() == canonical {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(AppError::Security(format!(
                "Target manifest rejects symbolic link: {}",
                entry.path().display()
            )));
        }
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(AppError::Validation(format!(
                "Target manifest rejects unsupported entry: {}",
                entry.path().display()
            )));
        }
        let metadata = entry.metadata().map_err(|error| {
            AppError::Io(format!(
                "Could not read target metadata for '{}': {error}",
                entry.path().display()
            ))
        })?;
        let relative = entry.path().strip_prefix(&canonical).map_err(|error| {
            AppError::Security(format!(
                "Target entry escaped '{}': {error}",
                canonical.display()
            ))
        })?;
        let relative_path = relative.to_string_lossy().replace('\\', "/");
        let modified_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        file_count = file_count.checked_add(1).ok_or_else(|| {
            AppError::Validation("Target metadata file count exceeds supported limit".to_string())
        })?;
        total_size_bytes = total_size_bytes
            .checked_add(metadata.len())
            .ok_or_else(|| {
                AppError::Validation("Target metadata total size overflow".to_string())
            })?;
        entries.push(TargetSnapshotEntry {
            relative_path,
            size_bytes: metadata.len(),
            modified_unix_ms,
        });
    }
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok((
        TargetSnapshot(entries),
        PayloadManifestMetadata {
            file_count,
            total_size_bytes: total_size_bytes.to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::TargetManifestIndexState;
    use crate::modules::ingestion::application::import_batch::payload_manifest::build_payload_manifest;

    #[test]
    fn refreshes_only_changed_target_manifest_entries() {
        let workspace = tempfile::tempdir().unwrap();
        let mods_root = workspace.path().join("Mods/character");
        let installed = mods_root.join("Ayaka");
        let source = workspace.path().join("source");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(installed.join("merged.ini"), "[TextureOverride]\nhash = aa").unwrap();
        std::fs::write(source.join("merged.ini"), "[TextureOverride]\nhash = aa").unwrap();

        let index = TargetManifestIndexState::new();
        let source_manifest = build_payload_manifest(&source, None).unwrap();
        assert!(index
            .find_existing_payload_match("batch", "gimi", &mods_root, &source_manifest)
            .unwrap()
            .is_some());

        std::fs::write(
            installed.join("merged.ini"),
            "[TextureOverride]\nhash = changed-hash",
        )
        .unwrap();
        assert!(index
            .find_existing_payload_match("batch", "gimi", &mods_root, &source_manifest)
            .unwrap()
            .is_none());
    }
}
