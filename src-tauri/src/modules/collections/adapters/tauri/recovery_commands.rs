use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus two IPC fields.
pub async fn resolve_recovery_task(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher_state: State<
        '_,
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    task_id: String,
    action: crate::modules::workspace::domain::task::RecoveryAction,
) -> Result<(), AppError> {
    if action == crate::modules::workspace::domain::task::RecoveryAction::Ignore {
        return crate::modules::workspace::application::recovery::resolve_recovery_task(
            crate::modules::workspace::application::recovery::RecoveryTaskRequest {
                pool: pool.inner(),
                config: config.inner(),
                watcher_state: watcher_state.inner(),
                coordinator: op_lock.inner(),
                task_id: &task_id,
                action,
            },
        )
        .await;
    }

    let task =
        crate::modules::workspace::adapters::sqlite::task::get_task_by_id(pool.inner(), &task_id)
            .await?
            .ok_or_else(|| AppError::Validation(format!("Task {task_id} not found")))?;
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &task.game_id,
        crate::modules::mutation::api::ImmutableMutationKind::Recovery,
    )?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &task.game_id,
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_nested_mutation_lease(&task.game_id, op_lock.inner())
        .await?;

    crate::modules::workspace::application::recovery::resolve_recovery_task(
        crate::modules::workspace::application::recovery::RecoveryTaskRequest {
            pool: pool.inner(),
            config: config.inner(),
            watcher_state: watcher_state.inner(),
            coordinator: op_lock.inner(),
            task_id: &task_id,
            action,
        },
    )
    .await?;
    crate::modules::reconciliation::api::enqueue_runtime_sync(
        &app,
        pool.inner(),
        &task.game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
    );
    drop(mutation_lease);
    Ok(())
}
