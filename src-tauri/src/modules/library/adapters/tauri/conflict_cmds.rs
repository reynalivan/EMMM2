use std::path::Path;

use tauri::{AppHandle, State};

use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::workspace::application::scanner::watcher::WatcherState;

use crate::modules::library::application::mods::core_ops::FolderConflictRename;
use crate::modules::settings::application::config::ConfigService;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;

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
    pub created_at: Option<u64>,
    pub modified_at: Option<u64>,
    pub files: Vec<String>,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FolderConflictMutationResult {
    pub reconcile: Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
    pub sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
}

fn settle_committed_conflict_trash(
    outcome: Result<
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
        AppError,
    >,
) -> FolderConflictMutationResult {
    let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(outcome);
    FolderConflictMutationResult {
        reconcile: settlement.reconcile,
        sync_warning: settlement.sync_warning,
    }
}

async fn rollback_conflict_rename_plan(
    state: &WatcherState,
    plan: &crate::modules::library::application::mods::core_ops::FolderConflictRenamePlan,
) -> Result<(), AppError> {
    let suppressor = state.suppressor.clone();
    let plan = plan.clone();
    tokio::task::spawn_blocking(move || {
        crate::modules::library::application::mods::core_ops::rollback_folder_conflict_rename_plan(
            &suppressor,
            &plan,
        )
    })
    .await?
}

fn mark_conflict_steps_rolled_back(
    mutation_lease: crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    step_count: usize,
) -> Result<(), AppError> {
    mutation_lease.begin_rollback()?;
    for sequence in (0..step_count).rev() {
        mutation_lease.mark_step_rolled_back(sequence as u32)?;
    }
    mutation_lease.finish_rollback()
}

#[specta::specta]
#[tauri::command]
pub async fn get_folder_conflict_details(
    config: State<'_, ConfigService>,
    game_id: String,
    paths: Vec<String>,
) -> Result<Vec<FolderConflictSummary>, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
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
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    group_id: String,
    renames: Vec<FolderConflictRename>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    AppError,
> {
    let paths = renames
        .iter()
        .map(|rename| rename.path.clone())
        .collect::<Vec<_>>();
    crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let canonical_root = mods_root
        .canonicalize()
        .map_err(|error| AppError::Security(format!("Invalid mods path: {error}")))?;
    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let rename_root = canonical_root.clone();
    let rename_game_id = game_id.clone();
    let rename_group_id = group_id.clone();
    let rename_requests = renames.clone();
    let rename_plan = tokio::task::spawn_blocking(move || {
        crate::modules::library::application::mods::core_ops::plan_folder_conflict_renames(
            &rename_root,
            &rename_game_id,
            &rename_group_id,
            &rename_requests,
        )
    })
    .await??;
    if rename_plan.is_empty() {
        return Err(AppError::Validation(
            "Folder conflict resolution did not produce any filesystem changes".to_string(),
        ));
    }
    let rewrites = rename_plan.rewrites();

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
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePathKind::Object
            } else {
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePathKind::Mod
            };
            Ok(crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePathUpdate { from, to, kind })
        })
        .collect::<Result<Vec<_>, AppError>>()
    {
        Ok(updates) => updates,
        Err(error) => return Err(error),
    };

    let journal_steps = rename_plan
        .journal_paths()
        .into_iter()
        .enumerate()
        .map(|(sequence, (old_path, stage_path, new_path))| {
            crate::modules::mutation::api::PlannedStep::rename(sequence as u32, old_path, new_path)
                .with_stage_path(stage_path)
        })
        .collect();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "folder-conflict-rename",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let suppressor = state.suppressor.clone();
    let apply_plan = rename_plan.clone();
    let apply_result = tokio::task::spawn_blocking(move || {
        crate::modules::library::application::mods::core_ops::apply_folder_conflict_rename_plan(
            &suppressor,
            &apply_plan,
        )
    })
    .await?;
    if let Err(error) = apply_result {
        if rename_plan.is_rolled_back() {
            mark_conflict_steps_rolled_back(mutation_lease, rewrites.len())?;
        } else {
            mutation_lease.fail(error.to_string())?;
        }
        return Err(error);
    }
    for sequence in 0..rewrites.len() {
        mutation_lease.mark_step_applied(sequence as u32)?;
    }

    let mut result =
        match crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
            &app,
            pool.inner(),
            &game_id,
            &mutation_lease,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                if let Err(rollback_error) =
                    rollback_conflict_rename_plan(&state, &rename_plan).await
                {
                    let combined = format!(
                        "{error}; conflict rename rollback failed: {rollback_error}"
                    );
                    mutation_lease.fail(combined.clone())?;
                    return Err(AppError::Io(combined));
                }
                if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                    &app,
                    pool.inner(),
                    &game_id,
                    &mutation_lease,
                )
                .await
                {
                    let combined = format!(
                        "{error}; rollback projection failed: {rollback_reconcile_error}"
                    );
                    mutation_lease.fail(combined.clone())?;
                    return Err(AppError::Io(combined));
                }
                mark_conflict_steps_rolled_back(mutation_lease, rewrites.len())?;
                return Err(error);
            }
        };
    mutation_lease.mark_db_committed()?;
    mutation_lease.commit()?;
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
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    path: String,
) -> Result<FolderConflictMutationResult, AppError> {
    let validated = validate_path(&config, &game_id, &path)?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let census_root = mods_root;
    let census_game_id = game_id.clone();
    let candidate_path = validated.as_ref().to_path_buf();
    let census_candidate_path = candidate_path.clone();
    let belongs_to_active_conflict = tokio::task::spawn_blocking(move || {
        let census =
            crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::collect_disk_identity_census(
                &census_root,
            )
            .map_err(|error| AppError::Internal(error.into_message()))?;
        Ok::<_, AppError>(
            crate::modules::reconciliation::application::disk_reconcile::identity_conflicts::detect_folder_name_conflicts_from_census(
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
    let prepared =
        crate::modules::library::application::mods::trash::prepare_trash_move(&candidate_path)?;
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "trash-folder-conflict-candidate",
            game_id.clone(),
            vec![crate::modules::mutation::api::PlannedStep::rename(
                0,
                prepared.source().to_path_buf(),
                prepared.quarantine().to_path_buf(),
            )],
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    if let Err(error) = prepared.execute(&state) {
        mutation_lease.mark_step_rolled_back(0)?;
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Err(error);
    }
    mutation_lease.mark_step_applied(0)?;
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
            &app,
            pool.inner(),
            &game_id,
            &mutation_lease,
        )
        .await;
    let reconcile = match reconcile {
        Ok(reconcile) if reconcile.status.applied() => reconcile,
        outcome => {
            let error = match outcome {
                Ok(reconcile) => AppError::Io(format!(
                    "Conflict trash reconcile requires attention: {:?}",
                    reconcile.status
                )),
                Err(error) => error,
            };
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) = prepared.rollback(&state) {
                let combined = format!("{error}; conflict trash rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            mutation_lease.mark_step_rolled_back(0)?;
            crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &game_id,
                &mutation_lease,
            )
            .await?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
    };
    mutation_lease.mark_db_committed()?;
    mutation_lease.commit()?;
    let mut result = settle_committed_conflict_trash(Ok(reconcile));
    if let Err(error) = prepared.finalize() {
        result.sync_warning = Some(
            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning {
                kind: crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::CleanupPending,
                message: error.to_string(),
            },
        );
    }
    Ok(result)
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
            .is_some_and(crate::modules::workspace::domain::normalizer::is_disabled_folder)
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
        created_at: scan.created_at,
        modified_at: scan.modified_at,
        files: scan.files,
        thumbnail_path: scan.thumbnail_path,
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
    created_at: Option<u64>,
    modified_at: Option<u64>,
    files: Vec<String>,
    thumbnail_path: Option<String>,
}

fn scan_folder(path: &Path) -> Result<FolderScan, AppError> {
    const MAX_SCANNED_FILES: usize = 100_000;
    let mut total_size: u64 = 0;
    let mut file_count = 0usize;
    let mut partial = false;
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    let mut thumbnail_path = None;

    let (created_at, modified_at) = path
        .metadata()
        .ok()
        .map(|m| {
            let created = m
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            let modified = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            (created, modified)
        })
        .unwrap_or((None, None));

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

        let name_lower = name.to_lowercase();
        if thumbnail_path.is_none()
            && (name_lower == "preview.png"
                || name_lower == "preview.jpg"
                || name_lower == "thumbnail.png"
                || name_lower == "thumbnail.jpg")
            && !name.contains('/')
            && !name.contains('\\')
        {
            thumbnail_path = Some(entry_path.to_string_lossy().to_string());
        }
        if files.len() < 15 {
            files.push(name.clone());
        }

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
        created_at,
        modified_at,
        files,
        thumbnail_path,
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
    crate::modules::workspace::adapters::sqlite::conflict::ignore_object_conflict(
        &pool, &game_id, &object_id, &mod_ids,
    )
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
    crate::modules::workspace::adapters::sqlite::conflict::revoke_object_conflict(
        &pool, &game_id, &object_id,
    )
    .await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn list_ignored_object_conflicts(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::modules::workspace::domain::conflicts::IgnoredConflict>, AppError> {
    let list =
        crate::modules::workspace::adapters::sqlite::conflict::list_ignored_object_conflicts(
            &pool, &game_id,
        )
        .await?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind;

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

        let game_lock = command.find("game_lock").unwrap();
        let plan = command.find("plan_folder_conflict_renames").unwrap();
        let lease = command.find("acquire_operation").unwrap();
        let rename = command.find("apply_folder_conflict_rename_plan").unwrap();
        let reconcile = command
            .find("run_full_internal_disk_reconcile_under_lease")
            .unwrap();
        let rollback = command.rfind("rollback_conflict_rename_plan").unwrap();
        let db_committed = command.find("mark_db_committed").unwrap();
        let commit = command.find("mutation_lease.commit").unwrap();

        assert!(game_lock < plan && plan < lease && lease < rename);
        assert!(rename < reconcile && reconcile < rollback);
        assert!(reconcile < db_committed && db_committed < commit);
        assert!(!command.contains("drop(mutation_lease)"));
    }
}
