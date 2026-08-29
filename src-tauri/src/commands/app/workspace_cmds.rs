use tauri::State;

use crate::domain::errors::AppError;
use crate::domain::workspace::{
    WorkspaceSwitchInput, WorkspaceSwitchResult, WorkspaceViewModel, WorkspaceViewModelInput,
};
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_view_model(
    app: tauri::AppHandle,
    input: WorkspaceViewModelInput,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::services::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<WorkspaceViewModel, AppError> {
    let game_id = input.filter.game_id.clone();
    let recovery_readiness = crate::services::disk_reconcile::emit::start_initial_disk_recovery(
        &app,
        pool.inner(),
        disk_reconcile_state.inner(),
        &game_id,
    );

    let mut workspace = crate::services::workspace_service::get_workspace_view_model_with_listing_mode(
        pool.inner(),
        input,
        matches!(
            recovery_readiness,
            crate::services::disk_reconcile::orchestrator::InitialRecoveryReadiness::Syncing { .. }
                | crate::services::disk_reconcile::orchestrator::InitialRecoveryReadiness::Unstarted { .. }
        ),
    )
    .await?;
    workspace.runtime.recovery_status = workspace_recovery_status(recovery_readiness);
    Ok(workspace)
}

fn workspace_recovery_status(
    readiness: crate::services::disk_reconcile::orchestrator::InitialRecoveryReadiness,
) -> crate::domain::workspace::WorkspaceRecoveryStatus {
    use crate::domain::workspace::WorkspaceRecoveryStatus;
    use crate::services::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    match readiness {
        InitialRecoveryReadiness::Ready { .. } => WorkspaceRecoveryStatus::Ready,
        InitialRecoveryReadiness::Failed { .. } => WorkspaceRecoveryStatus::Failed,
        InitialRecoveryReadiness::Unstarted { .. } | InitialRecoveryReadiness::Syncing { .. } => {
            WorkspaceRecoveryStatus::Syncing
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn execute_workspace_switch(
    app: tauri::AppHandle,
    input: WorkspaceSwitchInput,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
) -> Result<WorkspaceSwitchResult, AppError> {
    crate::services::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &input.game_id,
    )
    .await?;
    let op_guard = op_lock.acquire().await?;
    let game_id = input.game_id.clone();
    let result = crate::services::workspace_switch_service::execute_switch(
        input,
        config.inner(),
        pool.inner(),
        watcher_state.inner(),
        &op_guard,
    )
    .await;
    drop(op_guard);
    let mut result = match result {
        Ok(result) => result,
        Err(error) => {
            let reconcile =
                crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile(
                    &app,
                    pool.inner(),
                    &game_id,
                )
                .await;
            return match reconcile {
                Ok(_) => Err(error),
                Err(reconcile_error) => Err(AppError::Io(format!(
                    "{error}; convergence also failed: {reconcile_error}"
                ))),
            };
        }
    };
    if !result.changed_folder_paths.is_empty() {
        let settlement = crate::services::disk_reconcile::emit::settle_committed_reconcile(
            crate::services::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.changed_folder_paths.clone(),
            )
            .await,
        );
        result.sync_warning = settlement.sync_warning;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    #[test]
    fn pending_recovery_maps_to_syncing_workspace_runtime() {
        assert_eq!(
            workspace_recovery_status(InitialRecoveryReadiness::Syncing { generation: 7 }),
            crate::domain::workspace::WorkspaceRecoveryStatus::Syncing
        );
    }
}
