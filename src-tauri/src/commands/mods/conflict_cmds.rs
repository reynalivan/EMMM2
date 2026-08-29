use std::path::Path;

use tauri::{AppHandle, State};

use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;

use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::mods::core_ops::FolderConflictRename;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FolderConflictSummary {
    pub path: String,
    pub folder_name: String,
    pub is_enabled: bool,
    #[specta(type = f64)]
    pub total_size: u64,
    #[specta(type = f64)]
    pub file_count: usize,
    pub partial: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FolderConflictMutationResult {
    pub reconcile: Option<crate::services::disk_reconcile::types::DiskReconcileResult>,
    pub sync_warning: Option<crate::services::disk_reconcile::types::CommittedMutationSyncWarning>,
}

fn settle_committed_conflict_trash(
    outcome: Result<crate::services::disk_reconcile::types::DiskReconcileResult, AppError>,
) -> FolderConflictMutationResult {
    let settlement = crate::services::disk_reconcile::emit::settle_committed_reconcile(outcome);
    FolderConflictMutationResult {
        reconcile: settlement.reconcile,
        sync_warning: settlement.sync_warning,
    }
}

async fn rollback_conflict_renames(
    state: &WatcherState,
    rewrites: &[crate::services::mods::core_ops::FolderPathRename],
) -> Result<(), AppError> {
    let suppressor = state.suppressor.clone();
    let rewrites = rewrites.to_vec();
    tokio::task::spawn_blocking(move || {
        crate::services::mods::core_ops::rollback_folder_conflict_renames(&suppressor, &rewrites)
    })
    .await?
}

#[specta::specta]
#[tauri::command]
pub async fn get_folder_conflict_details(
    config: State<'_, ConfigService>,
    game_id: String,
    paths: Vec<String>,
) -> Result<Vec<FolderConflictSummary>, AppError> {
    let validated = crate::services::fs_utils::guard::validate_paths(&config, &game_id, &paths)?;
    tokio::task::spawn_blocking(move || {
        paths
            .into_iter()
            .zip(validated)
            .map(|(requested_path, validated_path)| {
                let validated_path = validated_path.to_string_lossy();
                let mut detail = scan_folder_summary(
                    &validated_path,
                    path_is_enabled(Path::new(validated_path.as_ref())),
                )?;
                detail.path = requested_path;
                Ok(detail)
            })
            .collect::<Result<Vec<_>, AppError>>()
    })
    .await?
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn resolve_folder_name_conflict(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::services::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    group_id: String,
    renames: Vec<FolderConflictRename>,
) -> Result<crate::services::disk_reconcile::types::DiskReconcileResult, AppError> {
    let paths = renames
        .iter()
        .map(|rename| rename.path.clone())
        .collect::<Vec<_>>();
    crate::services::fs_utils::guard::validate_paths(&config, &game_id, &paths)?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let canonical_root = mods_root
        .canonicalize()
        .map_err(|error| AppError::Security(format!("Invalid mods path: {error}")))?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(&game_id, op_lock.inner())
        .await?;
    let rename_root = canonical_root.clone();
    let rename_game_id = game_id.clone();
    let suppressor = state.suppressor.clone();
    let rewrites = tokio::task::spawn_blocking(move || {
        crate::services::mods::core_ops::apply_folder_conflict_renames(
            &rename_root,
            &rename_game_id,
            &suppressor,
            &group_id,
            &renames,
        )
    })
    .await??;

    let explicit_path_updates = match rewrites
        .iter()
        .map(|rewrite| {
            let from = Path::new(&rewrite.old_path)
                .strip_prefix(&canonical_root)
                .map_err(|_| {
                    AppError::Security("Conflict rewrite escapes the mods directory".to_string())
                })?
                .to_string_lossy()
                .to_string();
            let to = Path::new(&rewrite.new_path)
                .strip_prefix(&canonical_root)
                .map_err(|_| {
                    AppError::Security("Conflict rewrite escapes the mods directory".to_string())
                })?
                .to_string_lossy()
                .to_string();
            let kind = if Path::new(&from).components().count() == 1 {
                crate::services::disk_reconcile::types::DiskReconcilePathKind::Object
            } else {
                crate::services::disk_reconcile::types::DiskReconcilePathKind::Mod
            };
            Ok(crate::services::disk_reconcile::types::DiskReconcilePathUpdate { from, to, kind })
        })
        .collect::<Result<Vec<_>, AppError>>()
    {
        Ok(updates) => updates,
        Err(error) => {
            if let Err(rollback_error) = rollback_conflict_renames(&state, &rewrites).await {
                return Err(AppError::Io(format!(
                    "{error}; conflict rename rollback failed: {rollback_error}"
                )));
            }
            return Err(error);
        }
    };
    let mut result =
        match crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
            &app,
            pool.inner(),
            &game_id,
            &mutation_lease,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => match rollback_conflict_renames(&state, &rewrites).await {
                Ok(()) => return Err(error),
                Err(rollback_error) => {
                    return Err(AppError::Io(format!(
                        "{error}; conflict rename rollback failed: {rollback_error}"
                    )));
                }
            },
        };
    result.path_updates.extend(explicit_path_updates);
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn trash_folder_conflict_candidate(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::services::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    path: String,
) -> Result<FolderConflictMutationResult, AppError> {
    let validated = validate_path(&config, &game_id, &path)?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(&game_id, op_lock.inner())
        .await?;
    let census_root = mods_root;
    let census_game_id = game_id.clone();
    let candidate_path = validated.as_ref().to_path_buf();
    let census_candidate_path = candidate_path.clone();
    let belongs_to_active_conflict = tokio::task::spawn_blocking(move || {
        let census =
            crate::services::disk_reconcile::disk_snapshot::collect_disk_identity_census(
                &census_root,
            )
            .map_err(|error| AppError::Internal(error.into_message()))?;
        Ok::<_, AppError>(
            crate::services::disk_reconcile::identity_conflicts::detect_folder_name_conflicts_from_census(
                &census_game_id,
                &census,
            )
            .iter()
            .flat_map(|group| group.candidates.iter())
            .any(|candidate| {
                paths_refer_to_same_entry(&census_candidate_path, Path::new(&candidate.path))
            }),
        )
    })
    .await??;
    if !belongs_to_active_conflict {
        return Err(AppError::NotFound(
            "Folder conflict is stale or already resolved".to_string(),
        ));
    }
    let suppressor = state.suppressor.clone();
    tokio::task::spawn_blocking(move || {
        let _guard = suppressor.suppress_paths([candidate_path.as_path()]);
        crate::services::mods::trash::move_to_trash(&candidate_path)
    })
    .await??;
    Ok(settle_committed_conflict_trash(
        crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
            &app,
            pool.inner(),
            &game_id,
            &mutation_lease,
        )
        .await,
    ))
}

fn paths_refer_to_same_entry(canonical_path: &Path, candidate: &Path) -> bool {
    candidate
        .canonicalize()
        .is_ok_and(|canonical_candidate| canonical_candidate == canonical_path)
}

fn path_is_enabled(path: &Path) -> bool {
    !path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(crate::common::normalizer::is_disabled_folder)
    })
}

fn scan_folder_summary(
    path_str: &str,
    is_enabled: bool,
) -> Result<FolderConflictSummary, AppError> {
    let path = Path::new(path_str);
    let folder_name = folder_name_for_scan(path, path_str)?;
    let scan = scan_folder(path)?;

    Ok(FolderConflictSummary {
        path: path_str.to_string(),
        folder_name,
        is_enabled,
        total_size: scan.total_size,
        file_count: scan.file_count,
        partial: scan.partial,
        warnings: scan.warnings,
    })
}

fn folder_name_for_scan(path: &Path, path_str: &str) -> Result<String, AppError> {
    if !path.exists() || !path.is_dir() {
        return Err(AppError::Io(format!(
            "Path does not exist or is not a directory: {path_str}"
        )));
    }
    Ok(path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default())
}

struct FolderScan {
    total_size: u64,
    file_count: usize,
    partial: bool,
    warnings: Vec<String>,
}

fn scan_folder(path: &Path) -> Result<FolderScan, AppError> {
    const MAX_SCANNED_FILES: usize = 100_000;
    let mut total_size: u64 = 0;
    let mut file_count = 0usize;
    let mut partial = false;
    let mut warnings = Vec::new();

    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                partial = true;
                if warnings.is_empty() {
                    warnings.push(format!("Some entries could not be read: {error}"));
                }
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if file_count >= MAX_SCANNED_FILES {
            partial = true;
            warnings.push(format!(
                "Folder detail scan stopped after {MAX_SCANNED_FILES} files"
            ));
            break;
        }
        file_count += 1;
        let entry_path = entry.path();
        let name = entry_path
            .strip_prefix(path)
            .unwrap_or(entry_path)
            .to_string_lossy()
            .to_string();
        let size = match entry.metadata() {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                partial = true;
                if warnings.len() < 3 {
                    warnings.push(format!("Could not read metadata for '{name}': {error}"));
                }
                0
            }
        };
        total_size = total_size.saturating_add(size);
    }

    Ok(FolderScan {
        total_size,
        file_count,
        partial,
        warnings,
    })
}

#[specta::specta]
#[tauri::command]
pub async fn ignore_object_conflict(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
    mod_ids: Vec<String>,
) -> Result<(), AppError> {
    crate::repo::conflict_repo::ignore_object_conflict(&pool, &game_id, &object_id, &mod_ids)
        .await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn revoke_object_conflict(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
) -> Result<(), AppError> {
    crate::repo::conflict_repo::revoke_object_conflict(&pool, &game_id, &object_id).await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn list_ignored_object_conflicts(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::domain::conflicts::IgnoredConflict>, AppError> {
    let list = crate::repo::conflict_repo::list_ignored_object_conflicts(&pool, &game_id).await?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::disk_reconcile::types::CommittedMutationSyncWarningKind;

    #[test]
    fn conflict_membership_accepts_ordinary_and_canonical_path_spellings() {
        let temp = tempfile::TempDir::new().unwrap();
        let candidate = temp.path().join("DISABLED Candidate");
        std::fs::create_dir(&candidate).unwrap();
        let canonical = candidate.canonicalize().unwrap();

        assert!(paths_refer_to_same_entry(&canonical, &candidate));
        assert!(paths_refer_to_same_entry(&canonical, &canonical));
    }

    #[test]
    fn committed_trash_reconcile_failure_is_returned_as_typed_data() {
        let result = settle_committed_conflict_trash(Err(AppError::Io(
            "injected terminal reconcile failure".to_string(),
        )));

        assert!(result.reconcile.is_none());
        let warning = result
            .sync_warning
            .expect("committed trash must return a retryable projection warning");
        assert_eq!(
            warning.kind,
            CommittedMutationSyncWarningKind::ReconcileFailed
        );
        assert!(warning
            .message
            .contains("injected terminal reconcile failure"));
    }

    #[test]
    fn grouped_rename_keeps_the_mutation_lease_through_reconcile_and_rollback() {
        let source = include_str!("conflict_cmds.rs");
        let start = source
            .find("pub async fn resolve_folder_name_conflict")
            .unwrap();
        let end = source[start..]
            .find("pub async fn trash_folder_conflict_candidate")
            .map(|offset| start + offset)
            .unwrap();
        let command = &source[start..end];

        let lease = command.find("acquire_mutation_lease").unwrap();
        let rename = command.find("apply_folder_conflict_renames").unwrap();
        let reconcile = command
            .find("run_full_internal_disk_reconcile_under_lease")
            .unwrap();
        let rollback = command.rfind("rollback_conflict_renames").unwrap();

        assert!(lease < rename && rename < reconcile && reconcile < rollback);
        assert!(!command.contains("drop(mutation_lease)"));
    }
}
