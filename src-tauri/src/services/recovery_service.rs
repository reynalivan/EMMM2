//! Recovery for pipeline tasks that were interrupted mid-apply.
//!
//! Moved out of `commands::collections::cmds` so the command layer stays a thin
//! State-extraction wrapper and the orchestration is reachable from tests.

use sqlx::SqlitePool;

use crate::domain::errors::AppError;
use crate::domain::task::{PipelineTask, RecoveryAction, TaskStatus};
use crate::services::config::models::AppSettings;
use crate::services::config::ConfigService;
use crate::services::scanner::watcher::WatcherState;

/// Resolve one recovery task. The caller is responsible for holding the
/// operation lock: resuming an apply mutates the filesystem and must be
/// mutually excluded from concurrent runtime ops.
pub async fn resolve_recovery_task(
    pool: &SqlitePool,
    config: &ConfigService,
    watcher_state: &WatcherState,
    task_id: &str,
    action: RecoveryAction,
) -> Result<(), AppError> {
    log::info!(
        "Resolving recovery task {} with action {:?}",
        task_id,
        action
    );

    let task = crate::repo::task_repo::get_task_by_id(pool, task_id)
        .await?
        .ok_or_else(|| AppError::Validation(format!("Task {} not found", task_id)))?;

    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|game| game.id == task.game_id)
        .ok_or_else(|| AppError::Validation(format!("Game {} not found", task.game_id)))?
        .mod_path
        .clone();

    // The corridor comes from settings, not from the collection being recovered.
    // Deriving it from the data would make `validate_corridor` compare a value
    // with itself, letting a recovery apply an unsafe collection during Safe Mode.
    let is_safe = settings.safe_mode.enabled;

    match action {
        RecoveryAction::Retry => {
            retry_task(pool, watcher_state, &task, settings, mods_path, is_safe).await?;
            mark_task(pool, task_id, TaskStatus::Completed).await
        }
        RecoveryAction::Rollback => {
            rollback_task(pool, watcher_state, &task, settings, mods_path, is_safe).await?;
            mark_task(pool, task_id, TaskStatus::Completed).await
        }
        RecoveryAction::Ignore => mark_task(pool, task_id, TaskStatus::Failed).await,
    }
}

async fn mark_task(pool: &SqlitePool, task_id: &str, status: TaskStatus) -> Result<(), AppError> {
    crate::repo::task_repo::update_status(pool, task_id, status).await?;
    Ok(())
}

fn target_collection_id(task: &PipelineTask) -> Result<&str, AppError> {
    task.target_id
        .as_deref()
        .ok_or_else(|| AppError::Validation("Missing target collection ID".to_string()))
}

async fn retry_task(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    task: &PipelineTask,
    settings: AppSettings,
    mods_path: std::path::PathBuf,
    is_safe: bool,
) -> Result<(), AppError> {
    match task.task_type.as_str() {
        "apply_collection" => {
            // Existence is validated downstream by `validate_corridor`.
            let collection_id = target_collection_id(task)?;

            crate::services::collection_service::apply_collection(
                crate::services::collection_service::ApplyCollectionRequest {
                    pool,
                    game_id: &task.game_id,
                    collection_id,
                    is_safe,
                    mods_path,
                    suppressor: watcher_state.suppressor.clone(),
                    ignore_missing: true,
                    settings,
                    reconcile_lock: None,
                },
            )
            .await?;
            Ok(())
        }
        "switch_corridor" => {
            log::info!(
                "Ignoring legacy switch_corridor recovery task {} because mode switching has been removed",
                task.id
            );
            Ok(())
        }
        other => Err(AppError::Validation(format!(
            "Unsupported task type for retry: {}",
            other
        ))),
    }
}

async fn rollback_task(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    task: &PipelineTask,
    settings: AppSettings,
    mods_path: std::path::PathBuf,
    is_safe: bool,
) -> Result<(), AppError> {
    match task.task_type.as_str() {
        "switch_corridor" => {
            log::info!(
                "Ignoring legacy switch_corridor rollback task {} because mode switching has been removed",
                task.id
            );
            Ok(())
        }
        "apply_collection" => {
            let collection_id = target_collection_id(task)?;

            let rollback_collection_id =
                resolve_rollback_target(pool, task, collection_id, is_safe).await?;

            crate::services::collection_service::apply_collection(
                crate::services::collection_service::ApplyCollectionRequest {
                    pool,
                    game_id: &task.game_id,
                    collection_id: &rollback_collection_id,
                    is_safe,
                    mods_path,
                    suppressor: watcher_state.suppressor.clone(),
                    ignore_missing: true,
                    settings,
                    reconcile_lock: None,
                },
            )
            .await?;
            Ok(())
        }
        other => Err(AppError::Validation(format!(
            "Unsupported task type for rollback: {}",
            other
        ))),
    }
}

/// Pick a *different* collection to apply in place of the failed one.
///
/// NOTE: this is not a rollback in the restore-previous-state sense. Nothing
/// captures the runtime state that existed before the failed apply — `tasks`
/// has no snapshot column — so a hand-toggled runtime cannot be recovered and
/// is instead overwritten by whichever saved preset is chosen here. Real
/// rollback needs the pre-apply `ProjectedCollectionState` persisted on the
/// task row before the rename step runs.
async fn resolve_rollback_target(
    pool: &SqlitePool,
    task: &PipelineTask,
    collection_id: &str,
    is_safe: bool,
) -> Result<String, AppError> {
    let corridor_state = crate::repo::corridor_repo::get(pool, &task.game_id, is_safe).await?;

    let from_corridor = corridor_state
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
        .filter(|candidate| *candidate != collection_id)
        .map(|id| id.to_string());

    if let Some(existing_id) = from_corridor {
        return Ok(existing_id);
    }

    crate::services::corridor_service::resolve_restore_collection(pool, &task.game_id, is_safe)
        .await
        .ok()
        .flatten()
        .and_then(|(collection, _)| {
            if collection.id == collection_id {
                None
            } else {
                Some(collection.id)
            }
        })
        .ok_or_else(|| {
            AppError::Validation(
                "No rollback collection is available for this corridor".to_string(),
            )
        })
}

#[cfg(test)]
#[path = "tests/recovery_service_tests.rs"]
mod tests;
