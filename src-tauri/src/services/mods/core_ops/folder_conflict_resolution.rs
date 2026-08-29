use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::naming::validate_folder_base_name;
use crate::common::normalizer::is_disabled_folder;
use crate::domain::errors::AppError;
use crate::services::disk_reconcile::disk_snapshot::collect_disk_identity_census;
use crate::services::disk_reconcile::identity_conflicts::detect_folder_name_conflicts_from_census;
use crate::services::scanner::watcher::WatcherSuppressor;

/// One requested new base name for every candidate in a conflict group.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct FolderConflictRename {
    pub path: String,
    pub base_name: String,
}

/// An exact filesystem rewrite. Reconcile consumes the disk state after this
/// operation and remains the only projection/reference writer.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct FolderPathRename {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug)]
struct PendingRename {
    old_path: PathBuf,
    stage_path: PathBuf,
    target_path: PathBuf,
}

/// Compatibility hook for reconcile startup. Folder conflict batches no longer
/// persist recovery journals, so there is no durable state to replay.
pub fn recover_folder_conflict_journals(
    _mods_root: &Path,
    _suppressor: &std::sync::Arc<crate::services::scanner::watcher::WatcherSuppressor>,
) -> Result<(), AppError> {
    Ok(())
}

fn canonical(path: &Path) -> Result<PathBuf, AppError> {
    path.canonicalize()
        .map_err(|error| AppError::Security(format!("Invalid conflict path: {error}")))
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn target_folder_name(old_path: &Path, base_name: &str) -> String {
    let old_name = old_path.file_name().unwrap_or_default().to_string_lossy();
    if is_disabled_folder(&old_name) {
        format!("{}{base_name}", crate::DISABLED_PREFIX)
    } else {
        base_name.to_string()
    }
}

fn existing_sibling(parent: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(parent).ok()?.flatten().find_map(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            .then(|| entry.path())
    })
}

fn reject_occupied_destination(old_path: &Path, target_path: &Path) -> Result<(), AppError> {
    let parent = target_path
        .parent()
        .ok_or_else(|| AppError::Io("Conflict folder has no parent".to_string()))?;
    let target_name = target_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    if let Some(existing) = existing_sibling(parent, &target_name) {
        if path_key(&existing) != path_key(old_path) {
            return Err(AppError::Io(format!(
                "Conflict rename destination is not free: {}",
                existing.display()
            )));
        }
    }
    Ok(())
}

fn rollback_pending(pending: &[PendingRename]) -> Result<(), AppError> {
    let mut failures = Vec::new();
    for rename in pending {
        if rename.target_path.exists() && !rename.stage_path.exists() && !rename.old_path.exists() {
            if let Err(error) = fs::rename(&rename.target_path, &rename.stage_path) {
                failures.push(format!(
                    "'{}' to '{}': {error}",
                    rename.target_path.display(),
                    rename.stage_path.display()
                ));
            }
        }
    }
    for rename in pending.iter().rev() {
        if rename.stage_path.exists() && !rename.old_path.exists() {
            if let Err(error) = fs::rename(&rename.stage_path, &rename.old_path) {
                failures.push(format!(
                    "'{}' to '{}': {error}",
                    rename.stage_path.display(),
                    rename.old_path.display()
                ));
            }
        }
        if !rename.old_path.exists() || rename.stage_path.exists() {
            failures.push(format!(
                "rollback state for '{}' could not be verified",
                rename.old_path.display()
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Failed to roll back conflict rename(s): {}",
            failures.join("; ")
        )))
    }
}

/// Best-effort rollback for a terminal reconcile failure. This only covers the
/// current process; no journal is persisted, so interrupted batches leave their
/// staging folders for an explicit user repair rather than guessing at a swap.
pub fn rollback_folder_conflict_renames(
    suppressor: &Arc<WatcherSuppressor>,
    rewrites: &[FolderPathRename],
) -> Result<(), AppError> {
    let paths = rewrites
        .iter()
        .flat_map(|rewrite| [Path::new(&rewrite.old_path), Path::new(&rewrite.new_path)]);
    let _guard = suppressor.suppress_paths(paths);
    let pending = rewrites
        .iter()
        .enumerate()
        .map(|(index, rewrite)| PendingRename {
            old_path: PathBuf::from(&rewrite.old_path),
            stage_path: PathBuf::from(&rewrite.new_path).with_file_name(format!(
                ".emmm-conflict-stage-rollback-{index}-{}",
                uuid::Uuid::new_v4()
            )),
            target_path: PathBuf::from(&rewrite.new_path),
        })
        .collect::<Vec<_>>();
    rollback_pending(&pending)
}

/// Rename one complete current conflict group on disk and return every exact
/// rewrite. It deliberately does not touch SQLite or collection references.
pub fn apply_folder_conflict_renames(
    mods_root: &Path,
    game_id: &str,
    suppressor: &Arc<WatcherSuppressor>,
    group_id: &str,
    renames: &[FolderConflictRename],
) -> Result<Vec<FolderPathRename>, AppError> {
    let canonical_root = canonical(mods_root)?;
    let census = collect_disk_identity_census(&canonical_root)
        .map_err(|error| AppError::Internal(error.into_message()))?;
    let group = detect_folder_name_conflicts_from_census(game_id, &census)
        .into_iter()
        .find(|group| group.group_id == group_id)
        .ok_or_else(|| {
            AppError::NotFound("Folder conflict is stale or already resolved".to_string())
        })?;

    let expected_paths = group
        .candidates
        .iter()
        .map(|candidate| canonical(Path::new(&candidate.path)))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut requested = BTreeMap::<PathBuf, String>::new();
    for rename in renames {
        validate_folder_base_name(&rename.base_name)?;
        let old_path = canonical(Path::new(&rename.path))?;
        if !old_path.starts_with(&canonical_root) {
            return Err(AppError::Security(
                "Conflict path escapes the configured mods directory".to_string(),
            ));
        }
        if requested
            .insert(old_path, rename.base_name.clone())
            .is_some()
        {
            return Err(AppError::Security(
                "Rename set contains a duplicate conflict path".to_string(),
            ));
        }
    }
    if requested.keys().cloned().collect::<BTreeSet<_>>() != expected_paths {
        return Err(AppError::Security(
            "Rename set does not match the selected conflict group".to_string(),
        ));
    }

    let source_paths = requested.keys().cloned().collect::<BTreeSet<_>>();
    let mut final_identities = BTreeMap::<String, String>::new();
    for entry in &census.entries {
        let path = canonical(&entry.absolute_path)?;
        if !source_paths.contains(&path) {
            final_identities.insert(entry.folder_path_key.clone(), entry.folder_path.clone());
        }
    }

    let mut targets = BTreeSet::new();
    let mut planned = Vec::new();
    for (old_path, base_name) in &requested {
        let parent = old_path
            .parent()
            .ok_or_else(|| AppError::Io("Conflict folder has no parent".to_string()))?;
        let target_path = parent.join(target_folder_name(old_path, base_name));
        let target_key = path_key(&target_path);
        if !targets.insert(target_key) {
            return Err(AppError::Io(
                "Multiple renames have the same target".to_string(),
            ));
        }
        let relative = target_path
            .strip_prefix(&canonical_root)
            .map_err(|_| AppError::Security("Rename target escapes mods root".to_string()))?;
        let identity = crate::common::path_key::folder_path_key(&relative.to_string_lossy(), None);
        if let Some(existing) =
            final_identities.insert(identity, relative.to_string_lossy().to_string())
        {
            return Err(AppError::Io(format!(
                "Final folder identity conflicts with: {existing}"
            )));
        }
        if path_key(old_path) == path_key(&target_path) {
            continue;
        }
        reject_occupied_destination(old_path, &target_path)?;
        planned.push((old_path.clone(), target_path));
    }

    let pending = planned
        .iter()
        .enumerate()
        .map(|(index, (old_path, target_path))| PendingRename {
            old_path: old_path.clone(),
            stage_path: old_path.with_file_name(format!(
                ".emmm-conflict-stage-{index}-{}",
                uuid::Uuid::new_v4()
            )),
            target_path: target_path.clone(),
        })
        .collect::<Vec<_>>();
    let suppressed = pending
        .iter()
        .flat_map(|rename| [&rename.old_path, &rename.stage_path, &rename.target_path]);
    let _guard = suppressor.suppress_paths(suppressed);

    for (index, rename) in pending.iter().enumerate() {
        if let Err(error) = fs::rename(&rename.old_path, &rename.stage_path) {
            return match rollback_pending(&pending[..index]) {
                Ok(()) => Err(AppError::Io(format!(
                    "Failed to stage conflict rename: {error}"
                ))),
                Err(rollback_error) => Err(AppError::Io(format!(
                    "Failed to stage conflict rename: {error}; rollback failed: {rollback_error}"
                ))),
            };
        }
    }
    for rename in &pending {
        if let Err(error) = fs::rename(&rename.stage_path, &rename.target_path) {
            return match rollback_pending(&pending) {
                Ok(()) => Err(AppError::Io(format!(
                    "Failed to apply conflict rename: {error}"
                ))),
                Err(rollback_error) => Err(AppError::Io(format!(
                    "Failed to apply conflict rename: {error}; rollback failed: {rollback_error}"
                ))),
            };
        }
    }

    Ok(planned
        .into_iter()
        .map(|(old_path, new_path)| FolderPathRename {
            old_path: old_path.to_string_lossy().to_string(),
            new_path: new_path.to_string_lossy().to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::services::scanner::watcher::WatcherState;

    fn create_terminal_mod(path: &Path) {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("mod.ini"), "[TextureOverride]\nhash = abc\n").unwrap();
    }

    fn conflict_group(
        root: &Path,
    ) -> crate::services::disk_reconcile::types::FolderNameConflictGroup {
        detect_folder_name_conflicts_from_census(
            "game",
            &collect_disk_identity_census(root).unwrap(),
        )
        .remove(0)
    }

    #[test]
    fn batch_rename_preserves_prefix_and_returns_exact_rewrites() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let enabled = root.join("Alice").join("Blue");
        let disabled = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&enabled);
        create_terminal_mod(&disabled);
        let enabled_canonical = enabled.canonicalize().unwrap();
        let watcher = WatcherState::new();
        let group = conflict_group(root);

        let rewrites = apply_folder_conflict_renames(
            root,
            "game",
            &watcher.suppressor,
            &group.group_id,
            &[
                FolderConflictRename {
                    path: enabled.to_string_lossy().to_string(),
                    base_name: "Blue One".into(),
                },
                FolderConflictRename {
                    path: disabled.to_string_lossy().to_string(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap();

        assert_eq!(rewrites.len(), 2);
        assert_eq!(rewrites[0].old_path, enabled_canonical.to_string_lossy());
        assert_eq!(
            rewrites[0].new_path,
            enabled_canonical
                .parent()
                .unwrap()
                .join("Blue One")
                .to_string_lossy()
        );
        assert!(root.join("Alice").join("Blue One").is_dir());
        assert!(root.join("Alice").join("DISABLED Blue Two").is_dir());
    }

    #[test]
    fn batch_rename_rejects_windows_reserved_names_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let enabled = root.join("Alice").join("Blue");
        let disabled = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&enabled);
        create_terminal_mod(&disabled);
        let watcher = WatcherState::new();
        let group = conflict_group(root);

        let error = apply_folder_conflict_renames(
            root,
            "game",
            &watcher.suppressor,
            &group.group_id,
            &[
                FolderConflictRename {
                    path: enabled.to_string_lossy().to_string(),
                    base_name: "CON".into(),
                },
                FolderConflictRename {
                    path: disabled.to_string_lossy().to_string(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap_err();

        assert!(error.to_string().contains("reserved"));
        assert!(enabled.is_dir());
        assert!(disabled.is_dir());
    }

    #[test]
    fn batch_rename_rejects_disabled_prefix_in_base_name_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let enabled = root.join("Alice").join("Blue");
        let disabled = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&enabled);
        create_terminal_mod(&disabled);
        let watcher = WatcherState::new();
        let group = conflict_group(root);

        let error = apply_folder_conflict_renames(
            root,
            "game",
            &watcher.suppressor,
            &group.group_id,
            &[
                FolderConflictRename {
                    path: enabled.to_string_lossy().to_string(),
                    base_name: "DISABLED Blue One".into(),
                },
                FolderConflictRename {
                    path: disabled.to_string_lossy().to_string(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap_err();

        assert!(error.to_string().contains("DISABLED prefix"));
        assert!(enabled.is_dir());
        assert!(disabled.is_dir());
    }

    #[test]
    fn batch_rename_rejects_unchanged_normalized_identity_conflict() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let enabled = root.join("Alice").join("Blue");
        let disabled = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&enabled);
        create_terminal_mod(&disabled);
        let watcher = WatcherState::new();
        let group = conflict_group(root);

        let error = apply_folder_conflict_renames(
            root,
            "game",
            &watcher.suppressor,
            &group.group_id,
            &[
                FolderConflictRename {
                    path: enabled.to_string_lossy().to_string(),
                    base_name: "Blue".into(),
                },
                FolderConflictRename {
                    path: disabled.to_string_lossy().to_string(),
                    base_name: "Blue".into(),
                },
            ],
        )
        .unwrap_err();

        assert!(error.to_string().contains("identity conflicts"));
        assert!(enabled.is_dir());
        assert!(disabled.is_dir());
    }

    #[test]
    fn batch_rename_rejects_cycle_when_destinations_are_not_free() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let first = root.join("First");
        let second = root.join("Second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();

        let error = reject_occupied_destination(&first, &second).unwrap_err();

        assert!(error.to_string().contains("destination is not free"));
        assert!(first.is_dir());
        assert!(second.is_dir());
    }

    #[test]
    fn resolver_has_no_projection_or_collection_writer() {
        let source = include_str!("folder_conflict_resolution.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        let direct_projection_writer = ["persist_folder", "_conflict_rewrites"].concat();

        assert!(!production.contains(&direct_projection_writer));
        assert!(!production.contains("collection::"));
        assert!(!production.contains("sqlx::"));
        assert!(!production.contains("serde_json"));
        assert!(production.contains("collect_disk_identity_census"));
        assert!(!production.contains("collect_disk_projection"));
    }

    #[test]
    fn rollback_restores_every_renamed_folder() {
        let temp = tempfile::tempdir().unwrap();
        let old_one = temp.path().join("Blue");
        let old_two = temp.path().join("DISABLED Blue");
        let new_one = temp.path().join("Blue One");
        let new_two = temp.path().join("DISABLED Blue Two");
        fs::create_dir_all(&old_one).unwrap();
        fs::create_dir_all(&old_two).unwrap();
        fs::rename(&old_one, &new_one).unwrap();
        fs::rename(&old_two, &new_two).unwrap();
        let watcher = WatcherState::new();

        rollback_folder_conflict_renames(
            &watcher.suppressor,
            &[
                FolderPathRename {
                    old_path: old_one.to_string_lossy().to_string(),
                    new_path: new_one.to_string_lossy().to_string(),
                },
                FolderPathRename {
                    old_path: old_two.to_string_lossy().to_string(),
                    new_path: new_two.to_string_lossy().to_string(),
                },
            ],
        )
        .unwrap();

        assert!(old_one.is_dir());
        assert!(old_two.is_dir());
        assert!(!new_one.exists());
        assert!(!new_two.exists());
    }
}
