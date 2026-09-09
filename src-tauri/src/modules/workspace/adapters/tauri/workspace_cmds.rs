use tauri::State;

use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::domain::workspace::{
    WorkspaceSwitchInput, WorkspaceSwitchResult, WorkspaceViewModel, WorkspaceViewModelInput,
};
use crate::shared::errors::AppError;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_view_model(
    app: tauri::AppHandle,
    input: WorkspaceViewModelInput,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<WorkspaceViewModel, AppError> {
    let game_id = input.filter.game_id.clone();
    let recovery_readiness = crate::modules::reconciliation::application::disk_reconcile::emit::start_initial_disk_recovery(
        &app,
        pool.inner(),
        disk_reconcile_state.inner(),
        &game_id,
    );

    let mut workspace = crate::modules::workspace::application::workspace::get_workspace_view_model_with_listing_mode(
        pool.inner(),
        input,
        matches!(
            recovery_readiness,
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Syncing { .. }
                | crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Unstarted { .. }
        ),
    )
    .await?;
    workspace.runtime.recovery_status = workspace_recovery_status(recovery_readiness);
    Ok(workspace)
}

fn workspace_recovery_status(
    readiness: crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness,
) -> crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
    use crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus;

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
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchResult, AppError> {
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &input.game_id,
    )
    .await?;
    let game_id = input.game_id.clone();
    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    let prepared = crate::modules::workspace::application::workspace::switch::prepare_switch(
        &input,
        config.inner(),
        pool.inner(),
    )
    .await?;
    if let Some(result) = prepared.immediate_result() {
        return Ok(result);
    }
    let journal_steps = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, old_path, new_path)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, old_path, new_path)
        })
        .collect::<Vec<_>>();
    if journal_steps.is_empty() {
        let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
            )
            .await?;
        return prepared.execute(&app, &watcher_state);
    }
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "workspace-switch",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let result = match prepared.execute(&app, &watcher_state) {
        Ok(result) => result,
        Err(error) => {
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            mutation_lease.begin_rollback()?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
    };
    for (sequence, _, _) in prepared.journal_steps() {
        mutation_lease.mark_step_applied(sequence)?;
    }
    match crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        &app,
        pool.inner(),
        &game_id,
        &mutation_lease,
    )
    .await
    {
        Ok(reconcile) if reconcile.status.applied() => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            Ok(result)
        }
        outcome => {
            let error = match outcome {
                Ok(reconcile) => AppError::Io(format!(
                    "Workspace reconcile requires attention: {:?}",
                    reconcile.status
                )),
                Err(error) => error,
            };
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) = prepared.rollback(&watcher_state) {
                let combined = format!("{error}; workspace rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &game_id,
                &mutation_lease,
            )
            .await?;
            mutation_lease.finish_rollback()?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    #[test]
    fn pending_recovery_maps_to_syncing_workspace_runtime() {
        assert_eq!(
            workspace_recovery_status(InitialRecoveryReadiness::Syncing { generation: 7 }),
            crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus::Syncing
        );
    }
}
