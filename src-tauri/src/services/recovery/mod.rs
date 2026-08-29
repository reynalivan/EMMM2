//! Recovery for pipeline tasks that were interrupted mid-apply.
//!
//! Moved out of `commands::collections::cmds` so the command layer stays a thin
//! State-extraction wrapper and the orchestration is reachable from tests.

use sqlx::SqlitePool;

use crate::domain::errors::AppError;
use crate::domain::task::{PipelineTask, RecoveryAction, TaskStatus, TASK_TYPE_APPLY_COLLECTION};
use crate::services::config::models::AppSettings;
use crate::services::config::ConfigService;
use crate::services::scanner::watcher::WatcherState;

pub struct RecoveryTaskRequest<'a> {
    pub pool: &'a SqlitePool,
    pub config: &'a ConfigService,
    pub watcher_state: &'a WatcherState,
    pub task_id: &'a str,
    pub action: RecoveryAction,
}

pub async fn get_startup_recovery_tasks(pool: &SqlitePool) -> Result<Vec<PipelineTask>, AppError> {
    crate::repo::task::get_all_pending_tasks_global(pool).await
}

struct RecoveryApplyContext<'a> {
    pool: &'a SqlitePool,
    watcher_state: &'a WatcherState,
    settings: AppSettings,
    mods_path: std::path::PathBuf,
}

struct RecoveryRollbackTarget {
    collection_id: String,
    active_baseline_id: Option<String>,
}

/// Resolve one recovery task. The caller is responsible for holding the
/// operation lock: resuming an apply mutates the filesystem and must be
/// mutually excluded from concurrent runtime ops.
pub async fn resolve_recovery_task(request: RecoveryTaskRequest<'_>) -> Result<(), AppError> {
    let RecoveryTaskRequest {
        pool,
        config,
        watcher_state,
        task_id,
        action,
    } = request;
    log::info!(
        "Resolving recovery task {} with action {:?}",
        task_id,
        action
    );

    let task = crate::repo::task::get_task_by_id(pool, task_id)
        .await?
        .ok_or_else(|| AppError::Validation(format!("Task {} not found", task_id)))?;

    if action == RecoveryAction::Ignore {
        return settle_ignored_task(pool, task_id).await;
    }

    let claimed = crate::repo::task::compare_and_set_status(
        pool,
        task_id,
        TaskStatus::Pending,
        TaskStatus::Running,
    )
    .await?;
    if !claimed {
        return Err(AppError::Validation(format!(
            "Recovery task '{task_id}' is already claimed or settled"
        )));
    }

    let result = resolve_claimed_task(pool, config, watcher_state, &task, action).await;
    if result.is_err() {
        match crate::repo::task::compare_and_set_status(
            pool,
            task_id,
            TaskStatus::Running,
            TaskStatus::Pending,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => log::error!(
                "Recovery task '{task_id}' failed but was no longer RUNNING during release"
            ),
            Err(error) => log::error!(
                "Recovery task '{task_id}' failed and could not be released to PENDING: {error}"
            ),
        }
    }
    result
}

async fn settle_ignored_task(pool: &SqlitePool, task_id: &str) -> Result<(), AppError> {
    let settled = crate::repo::task::compare_and_set_status(
        pool,
        task_id,
        TaskStatus::Pending,
        TaskStatus::Failed,
    )
    .await?;
    if !settled {
        return Err(AppError::Validation(format!(
            "Recovery task '{task_id}' is already claimed or settled"
        )));
    }
    Ok(())
}

async fn resolve_claimed_task(
    pool: &SqlitePool,
    config: &ConfigService,
    watcher_state: &WatcherState,
    task: &PipelineTask,
    action: RecoveryAction,
) -> Result<(), AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|game| game.id == task.game_id)
        .ok_or_else(|| AppError::Validation(format!("Game {} not found", task.game_id)))?
        .mod_path
        .clone();

    let apply_context = RecoveryApplyContext {
        pool,
        watcher_state,
        settings,
        mods_path,
    };

    match action {
        RecoveryAction::Retry => retry_task(apply_context, task).await,
        RecoveryAction::Rollback => rollback_task(apply_context, task).await,
        RecoveryAction::Ignore => Err(AppError::Validation(
            "Ignore does not run through the claimed recovery path".to_string(),
        )),
    }
}

fn target_collection_id(task: &PipelineTask) -> Result<&str, AppError> {
    task.target_id
        .as_deref()
        .ok_or_else(|| AppError::Validation("Missing target collection ID".to_string()))
}

async fn retry_task(
    context: RecoveryApplyContext<'_>,
    task: &PipelineTask,
) -> Result<(), AppError> {
    match task.task_type.as_str() {
        TASK_TYPE_APPLY_COLLECTION => {
            // Existence is validated by the collection apply pipeline.
            let collection_id = target_collection_id(task)?;
            let final_active_collection_id =
                crate::services::collection::valid_active_baseline(
                    context.pool,
                    &task.game_id,
                    task.final_active_collection_id.as_deref(),
                )
                .await?;

            crate::services::collection::apply_collection_with_existing_task(
                crate::services::collection::ApplyCollectionRequest {
                    pool: context.pool,
                    game_id: &task.game_id,
                    collection_id,
                    capture_last_changes: false,
                    mods_path: context.mods_path,
                    suppressor: context.watcher_state.suppressor.clone(),
                    ignore_missing: true,
                    settings: context.settings,
                },
                &task.id,
                final_active_collection_id,
            )
            .await?;
            Ok(())
        }
        other => Err(AppError::Validation(format!(
            "Unsupported task type for retry: {}",
            other
        ))),
    }
}

async fn rollback_task(
    context: RecoveryApplyContext<'_>,
    task: &PipelineTask,
) -> Result<(), AppError> {
    match task.task_type.as_str() {
        TASK_TYPE_APPLY_COLLECTION => {
            let rollback = resolve_rollback_target(context.pool, task).await?;

            crate::services::collection::apply_collection_with_existing_task(
                crate::services::collection::ApplyCollectionRequest {
                    pool: context.pool,
                    game_id: &task.game_id,
                    collection_id: &rollback.collection_id,
                    capture_last_changes: false,
                    mods_path: context.mods_path,
                    suppressor: context.watcher_state.suppressor.clone(),
                    ignore_missing: true,
                    settings: context.settings,
                },
                &task.id,
                rollback.active_baseline_id,
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

async fn resolve_rollback_target(
    pool: &SqlitePool,
    task: &PipelineTask,
) -> Result<RecoveryRollbackTarget, AppError> {
    let collection_id = task.rollback_collection_id.clone().ok_or_else(|| {
        AppError::Validation("Recovery task has no stored rollback collection".to_string())
    })?;
    let rollback_exists = crate::repo::collection::get_by_id(pool, &collection_id)
        .await?
        .is_some_and(|collection| collection.game_id == task.game_id);
    if !rollback_exists {
        return Err(AppError::Validation(format!(
            "Stored rollback collection does not exist: {collection_id}"
        )));
    }
    let active_baseline_id = crate::services::collection::valid_active_baseline(
        pool,
        &task.game_id,
        task.rollback_active_collection_id.as_deref(),
    )
    .await?;

    Ok(RecoveryRollbackTarget {
        collection_id,
        active_baseline_id,
    })
}

#[cfg(test)]
mod tests;
