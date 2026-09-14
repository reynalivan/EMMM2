use std::time::Instant;
use tauri::State;

use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::emit::{
    require_applied_reconcile, settle_committed_reconcile,
};
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::application::workspace::switch::PreparedWorkspaceExecutionError;
use crate::modules::workspace::domain::workspace::{
    WorkspacePreviewInput, WorkspacePreviewResult, WorkspaceStructureInput,
    WorkspaceStructureViewModel, WorkspaceSwitchInput, WorkspaceSwitchResult,
};
use crate::shared::errors::AppError;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_structure(
    app: tauri::AppHandle,
    input: WorkspaceStructureInput,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<WorkspaceStructureViewModel, AppError> {
    let game_id = input.filter.game_id.clone();
    let recovery_readiness = crate::modules::reconciliation::application::disk_reconcile::emit::start_initial_disk_recovery(
        &app,
        pool.inner(),
        disk_reconcile_state.inner(),
        &game_id,
    );

    let mut workspace = crate::modules::workspace::application::workspace::get_workspace_structure_with_listing_mode(
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

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_preview(
    input: WorkspacePreviewInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<WorkspacePreviewResult, AppError> {
    crate::modules::workspace::application::workspace::get_workspace_preview(pool.inner(), input)
        .await
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
    let started_at = Instant::now();
    let preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_for_paths(
        &app,
        pool.inner(),
        &input.game_id,
        None,
    )
    .await?;
    let preflight_elapsed = started_at.elapsed();
    let game_id = input.game_id.clone();
    let lock_wait_started_at = Instant::now();
    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    let lock_wait_elapsed = lock_wait_started_at.elapsed();
    let prepare_started_at = Instant::now();
    let prepared = crate::modules::workspace::application::workspace::switch::prepare_switch(
        &input,
        config.inner(),
        pool.inner(),
    )
    .await?;
    let prepare_elapsed = prepare_started_at.elapsed();
    if let Some(result) = prepared.immediate_result() {
        log::debug!(
            "workspace switch timing outcome=immediate preflight_ms={} lock_wait_ms={} prepare_ms={} total_ms={}",
            preflight_elapsed.as_millis(),
            lock_wait_elapsed.as_millis(),
            prepare_elapsed.as_millis(),
            started_at.elapsed().as_millis(),
        );
        return Ok(result);
    }
    let planned_renames = prepared.journal_steps();
    let preflight_paths = planned_renames
        .iter()
        .flat_map(|(_, old_path, new_path)| {
            [
                old_path.to_string_lossy().into_owned(),
                new_path.to_string_lossy().into_owned(),
            ]
        })
        .collect::<Vec<_>>();
    if crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
        &preflight.folder_conflicts,
        &preflight_paths,
    ) {
        return Err(
            crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error(),
        );
    }
    let journal_steps = planned_renames
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
        let execute_started_at = Instant::now();
        let result = prepared.execute(&app, &watcher_state);
        log::debug!(
            "workspace switch timing outcome=exempt preflight_ms={} lock_wait_ms={} prepare_ms={} execute_ms={} total_ms={}",
            preflight_elapsed.as_millis(),
            lock_wait_elapsed.as_millis(),
            prepare_elapsed.as_millis(),
            execute_started_at.elapsed().as_millis(),
            started_at.elapsed().as_millis(),
        );
        return result;
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
    let execute_started_at = Instant::now();
    let mut result = match prepared.execute_with_outcome(&app, &watcher_state) {
        Ok(result) => result,
        Err(PreparedWorkspaceExecutionError::Apply(error)) => {
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            mutation_lease.begin_rollback()?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
        Err(PreparedWorkspaceExecutionError::Compensation(error)) => {
            mutation_lease.fail(format!(
                "Workspace compensation failed; recovery is required: {error}"
            ))?;
            return Err(error);
        }
    };
    let execute_elapsed = execute_started_at.elapsed();
    for (sequence, _, _) in prepared.journal_steps() {
        mutation_lease.mark_step_applied(sequence)?;
    }
    let reconcile_started_at = Instant::now();
    match crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        &app,
        pool.inner(),
        &game_id,
        &mutation_lease,
    )
    .await.and_then(require_applied_reconcile)
    {
        Ok(reconcile) => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            result.sync_warning = settle_committed_reconcile(Ok(reconcile)).sync_warning;
            log::debug!(
                "workspace switch timing outcome=applied preflight_ms={} lock_wait_ms={} prepare_ms={} execute_ms={} reconcile_ms={} total_ms={}",
                preflight_elapsed.as_millis(),
                lock_wait_elapsed.as_millis(),
                prepare_elapsed.as_millis(),
                execute_elapsed.as_millis(),
                reconcile_started_at.elapsed().as_millis(),
                started_at.elapsed().as_millis(),
            );
            Ok(result)
        }
        Err(error) => {
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) = prepared.rollback(&watcher_state) {
                let combined = format!("{error}; workspace rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for (sequence, _, _) in prepared.journal_steps() {
                mutation_lease.mark_step_rolled_back(sequence)?;
            }
            if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &game_id,
                &mutation_lease,
            )
            .await.and_then(require_applied_reconcile) {
                let combined = format!("{error}; rollback projection failed: {rollback_reconcile_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
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
