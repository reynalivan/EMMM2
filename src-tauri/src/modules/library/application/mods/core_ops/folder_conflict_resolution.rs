use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::naming::validate_folder_base_name;
use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
    collect_disk_identity_census, filesystem_identity,
};
use crate::modules::reconciliation::application::disk_reconcile::identity_conflicts::detect_folder_name_conflicts_from_census;
use crate::modules::workspace::application::scanner::watcher::WatcherSuppressor;
use crate::modules::workspace::domain::normalizer::is_disabled_folder;
use crate::platform::fs::rename::rename_no_replace;
use crate::shared::errors::AppError;

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

#[derive(Debug, Clone)]
struct PendingRename {
    old_path: PathBuf,
    stage_path: PathBuf,
    target_path: PathBuf,
    expected_identity: String,
}

#[derive(Debug, Clone)]
pub struct FolderConflictRenamePlan {
    pending: Vec<PendingRename>,
}

impl FolderConflictRenamePlan {
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn journal_paths(&self) -> Vec<(PathBuf, PathBuf, PathBuf, String)> {
        self.pending
            .iter()
            .map(|rename| {
                (
                    rename.old_path.clone(),
                    rename.stage_path.clone(),
                    rename.target_path.clone(),
                    rename.expected_identity.clone(),
                )
            })
            .collect()
    }

    pub fn rewrites(&self) -> Vec<FolderPathRename> {
        self.pending
            .iter()
            .map(|rename| FolderPathRename {
                old_path: rename.old_path.to_string_lossy().to_string(),
                new_path: rename.target_path.to_string_lossy().to_string(),
            })
            .collect()
    }

    pub fn is_rolled_back(&self) -> bool {
        self.pending.iter().all(|rename| {
            has_expected_identity(&rename.old_path, &rename.expected_identity)
                && !rename.stage_path.exists()
                && !rename.target_path.exists()
        })
    }
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

fn has_expected_identity(path: &Path, expected: &str) -> bool {
    filesystem_identity(path).as_deref() == Some(expected)
}

fn require_expected_identity(path: &Path, expected: &str) -> Result<(), AppError> {
    if has_expected_identity(path, expected) {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Conflict folder changed after planning: {}",
            path.display()
        )))
    }
}

fn require_free(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(AppError::Io(format!(
            "Conflict rename destination is not free: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(format!(
            "Could not inspect conflict destination {}: {error}",
            path.display()
        ))),
    }
}

fn target_folder_name(old_path: &Path, base_name: &str) -> String {
    let old_name = old_path.file_name().unwrap_or_default().to_string_lossy();
    if is_disabled_folder(&old_name) {
        format!("{}{base_name}", crate::DISABLED_PREFIX)
    } else {
        base_name.to_string()
    }
}

type SiblingIndex = BTreeMap<String, PathBuf>;

fn sibling_index(parent: &Path) -> Option<SiblingIndex> {
    fs::read_dir(parent).ok().map(|entries| {
        entries
            .flatten()
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().to_ascii_lowercase(),
                    entry.path(),
                )
            })
            .collect()
    })
}

#[cfg(test)]
fn reject_occupied_destination(old_path: &Path, target_path: &Path) -> Result<(), AppError> {
    let parent = target_path
        .parent()
        .ok_or_else(|| AppError::Io("Conflict folder has no parent".to_string()))?;
    reject_occupied_destination_from_index(old_path, target_path, sibling_index(parent).as_ref())
}

fn reject_occupied_destination_from_index(
    old_path: &Path,
    target_path: &Path,
    siblings: Option<&SiblingIndex>,
) -> Result<(), AppError> {
    let target_name = target_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    if let Some(existing) =
        siblings.and_then(|siblings| siblings.get(&target_name.to_ascii_lowercase()))
    {
        if path_key(existing) != path_key(old_path) {
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
            if let Err(error) = require_expected_identity(
                &rename.target_path,
                &rename.expected_identity,
            )
            .and_then(|_| {
                rename_no_replace(&rename.target_path, &rename.stage_path).map_err(AppError::from)
            }) {
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
            if let Err(error) = require_expected_identity(
                &rename.stage_path,
                &rename.expected_identity,
            )
            .and_then(|_| {
                rename_no_replace(&rename.stage_path, &rename.old_path).map_err(AppError::from)
            }) {
                failures.push(format!(
                    "'{}' to '{}': {error}",
                    rename.stage_path.display(),
                    rename.old_path.display()
                ));
            }
        }
        if !has_expected_identity(&rename.old_path, &rename.expected_identity)
            || rename.stage_path.exists()
            || rename.target_path.exists()
        {
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

/// Rename one complete current conflict group on disk and return every exact
/// rewrite. It deliberately does not touch SQLite or collection references.
pub fn plan_folder_conflict_renames(
    mods_root: &Path,
    game_id: &str,
    group_id: &str,
    renames: &[FolderConflictRename],
) -> Result<FolderConflictRenamePlan, AppError> {
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
    let mut sibling_indexes = BTreeMap::<PathBuf, Option<SiblingIndex>>::new();
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
        let identity = crate::shared::path_key::folder_path_key(&relative.to_string_lossy(), None);
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
        let siblings = sibling_indexes
            .entry(parent.to_path_buf())
            .or_insert_with(|| sibling_index(parent));
        reject_occupied_destination_from_index(old_path, &target_path, siblings.as_ref())?;
        planned.push((old_path.clone(), target_path));
    }

    let pending = planned
        .iter()
        .enumerate()
        .map(|(index, (old_path, target_path))| {
            Ok(PendingRename {
                old_path: old_path.clone(),
                stage_path: old_path.with_file_name(format!(
                    ".emmm-conflict-stage-{index}-{}",
                    uuid::Uuid::new_v4()
                )),
                target_path: target_path.clone(),
                expected_identity: filesystem_identity(old_path).ok_or_else(|| {
                    AppError::Io(format!(
                        "Could not identify conflict source: {}",
                        old_path.display()
                    ))
                })?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(FolderConflictRenamePlan { pending })
}

pub fn apply_folder_conflict_rename_plan(
    suppressor: &Arc<WatcherSuppressor>,
    plan: &FolderConflictRenamePlan,
) -> Result<Vec<FolderPathRename>, AppError> {
    let suppressed = plan
        .pending
        .iter()
        .flat_map(|rename| [&rename.old_path, &rename.stage_path, &rename.target_path]);
    let _guard = suppressor.suppress_paths(suppressed);

    for (index, rename) in plan.pending.iter().enumerate() {
        if let Err(error) = require_expected_identity(&rename.old_path, &rename.expected_identity)
            .and_then(|_| require_free(&rename.stage_path))
            .and_then(|_| {
                rename_no_replace(&rename.old_path, &rename.stage_path).map_err(AppError::from)
            })
        {
            return match rollback_pending(&plan.pending[..index]) {
                Ok(()) => Err(AppError::Io(format!(
                    "Failed to stage conflict rename: {error}"
                ))),
                Err(rollback_error) => Err(AppError::Io(format!(
                    "Failed to stage conflict rename: {error}; rollback failed: {rollback_error}"
                ))),
            };
        }
    }
    for rename in &plan.pending {
        if let Err(error) = require_expected_identity(&rename.stage_path, &rename.expected_identity)
            .and_then(|_| require_free(&rename.target_path))
            .and_then(|_| {
                rename_no_replace(&rename.stage_path, &rename.target_path).map_err(AppError::from)
            })
        {
            return match rollback_pending(&plan.pending) {
                Ok(()) => Err(AppError::Io(format!(
                    "Failed to apply conflict rename: {error}"
                ))),
                Err(rollback_error) => Err(AppError::Io(format!(
                    "Failed to apply conflict rename: {error}; rollback failed: {rollback_error}"
                ))),
            };
        }
    }

    Ok(plan.rewrites())
}

pub fn rollback_folder_conflict_rename_plan(
    suppressor: &Arc<WatcherSuppressor>,
    plan: &FolderConflictRenamePlan,
) -> Result<(), AppError> {
    let suppressed = plan
        .pending
        .iter()
        .flat_map(|rename| [&rename.old_path, &rename.stage_path, &rename.target_path]);
    let _guard = suppressor.suppress_paths(suppressed);
    rollback_pending(&plan.pending)
}

pub fn apply_folder_conflict_renames(
    mods_root: &Path,
    game_id: &str,
    suppressor: &Arc<WatcherSuppressor>,
    group_id: &str,
    renames: &[FolderConflictRename],
) -> Result<Vec<FolderPathRename>, AppError> {
    let plan = plan_folder_conflict_renames(mods_root, game_id, group_id, renames)?;
    apply_folder_conflict_rename_plan(suppressor, &plan)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::modules::workspace::application::scanner::watcher::WatcherState;

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn atomic_rename_preserves_an_occupied_directory() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        let destination_identity = filesystem_identity(&destination).unwrap();

        rename_no_replace(&source, &destination).expect_err("destination must not be replaced");

        assert!(source.is_dir());
        assert_eq!(
            filesystem_identity(&destination),
            Some(destination_identity)
        );
    }

    fn create_terminal_mod(path: &Path) {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("mod.ini"), "[TextureOverride]\nhash = abc\n").unwrap();
    }

    fn conflict_group(
        root: &Path,
    ) -> crate::modules::reconciliation::application::disk_reconcile::types::FolderNameConflictGroup
    {
        detect_folder_name_conflicts_from_census(
            "game",
            &collect_disk_identity_census(root).unwrap(),
        )
        .remove(0)
    }

    #[test]
    fn prepared_conflict_rename_rejects_replacement_source() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let source = root.join("Alice").join("Blue");
        let other = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&source);
        create_terminal_mod(&other);
        let group = conflict_group(root);
        let plan = plan_folder_conflict_renames(
            root,
            "game",
            &group.group_id,
            &[
                FolderConflictRename {
                    path: source.to_string_lossy().into_owned(),
                    base_name: "Blue One".into(),
                },
                FolderConflictRename {
                    path: other.to_string_lossy().into_owned(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap();
        for (old_path, _, _, expected_identity) in plan.journal_paths() {
            assert_eq!(filesystem_identity(&old_path), Some(expected_identity));
        }
        let parked = root.join("parked");
        fs::rename(&source, &parked).unwrap();
        create_terminal_mod(&source);

        let error = apply_folder_conflict_rename_plan(&WatcherState::new().suppressor, &plan)
            .expect_err("replacement source must be rejected");
        assert!(error.to_string().contains("changed"));
        assert!(source.exists());
        assert!(parked.exists());
        assert!(other.exists());
        assert!(!root.join("Alice").join("Blue One").exists());
    }

    #[test]
    fn conflict_rollback_rejects_replacement_target() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let source = root.join("Alice").join("Blue");
        let other = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&source);
        create_terminal_mod(&other);
        let group = conflict_group(root);
        let plan = plan_folder_conflict_renames(
            root,
            "game",
            &group.group_id,
            &[
                FolderConflictRename {
                    path: source.to_string_lossy().into_owned(),
                    base_name: "Blue One".into(),
                },
                FolderConflictRename {
                    path: other.to_string_lossy().into_owned(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap();
        let watcher = WatcherState::new();
        apply_folder_conflict_rename_plan(&watcher.suppressor, &plan).unwrap();
        let target = root.join("Alice").join("Blue One");
        let parked = root.join("parked");
        fs::rename(&target, &parked).unwrap();
        create_terminal_mod(&target);

        let error = rollback_folder_conflict_rename_plan(&watcher.suppressor, &plan)
            .expect_err("replacement target must be rejected");
        assert!(error.to_string().contains("changed"));
        assert!(target.exists());
        assert!(parked.exists());
        assert!(!source.exists());
    }

    #[test]
    fn prepared_conflict_rename_rejects_late_destination() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let source = root.join("Alice").join("Blue");
        let other = root.join("Alice").join("DISABLED Blue");
        create_terminal_mod(&source);
        create_terminal_mod(&other);
        let group = conflict_group(root);
        let plan = plan_folder_conflict_renames(
            root,
            "game",
            &group.group_id,
            &[
                FolderConflictRename {
                    path: source.to_string_lossy().into_owned(),
                    base_name: "Blue One".into(),
                },
                FolderConflictRename {
                    path: other.to_string_lossy().into_owned(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap();
        let late_target = root.join("Alice").join("Blue One");
        create_terminal_mod(&late_target);

        let error = apply_folder_conflict_rename_plan(&WatcherState::new().suppressor, &plan)
            .expect_err("late destination must be preserved");
        assert!(error.to_string().contains("not free"));
        assert!(source.exists());
        assert!(other.exists());
        assert!(late_target.join("mod.ini").exists());
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
        let old_one = temp.path().join("Alice").join("Blue");
        let old_two = temp.path().join("Alice").join("DISABLED Blue");
        let new_one = temp.path().join("Alice").join("Blue One");
        let new_two = temp.path().join("Alice").join("DISABLED Blue Two");
        create_terminal_mod(&old_one);
        create_terminal_mod(&old_two);
        let group = conflict_group(temp.path());
        let plan = plan_folder_conflict_renames(
            temp.path(),
            "game",
            &group.group_id,
            &[
                FolderConflictRename {
                    path: old_one.to_string_lossy().into_owned(),
                    base_name: "Blue One".into(),
                },
                FolderConflictRename {
                    path: old_two.to_string_lossy().into_owned(),
                    base_name: "Blue Two".into(),
                },
            ],
        )
        .unwrap();
        let watcher = WatcherState::new();
        apply_folder_conflict_rename_plan(&watcher.suppressor, &plan).unwrap();
        rollback_folder_conflict_rename_plan(&watcher.suppressor, &plan).unwrap();

        assert!(old_one.is_dir());
        assert!(old_two.is_dir());
        assert!(!new_one.exists());
        assert!(!new_two.exists());
    }
}
