use crate::modules::library::application::mods::bulk;
use crate::modules::library::application::mods::info_json;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::shared::errors::AppError;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, State};

/// Cooperative cancel for the two bulk actions that walk the filesystem one
/// folder at a time. A single flag is enough: `OperationLock` already
/// serializes bulk runs, so two batches are never in flight together.
#[derive(Default)]
pub struct BulkCancelState(AtomicBool);

impl BulkCancelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears a cancel left over from an earlier batch and hands back the flag.
    /// Callers already hold the operation lock, so this cannot wipe a cancel
    /// aimed at a run that is still going.
    fn begin(&self) -> &AtomicBool {
        self.0.store(false, Ordering::SeqCst);
        &self.0
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn apply_committed_reconcile(
    result: &mut bulk::BulkResult,
    settlement: crate::modules::reconciliation::application::disk_reconcile::emit::CommittedReconcileSettlement,
) {
    if let Some(reconcile) = settlement.reconcile {
        result
            .collection_impact
            .merge(reconcile.collection_reference_impact);
    }
    result.sync_warning = settlement.sync_warning;
}

/// Stop the running bulk toggle/delete after the item in flight. Work already
/// done stays done — the trailing reconcile still converges the DB.
#[specta::specta]
#[tauri::command]
pub async fn bulk_cancel(cancel_state: State<'_, BulkCancelState>) -> Result<(), AppError> {
    cancel_state.cancel();
    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_toggle_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, AppError> {
    // Security validation for all paths
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;
    if validated.is_empty() {
        return Ok(bulk::BulkResult::new(Vec::new(), Vec::new()));
    }

    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let validated_paths = validated
        .iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect::<Vec<_>>();
    let prepared = bulk::prepare_bulk_toggle(&validated_paths, enable);
    let planned_steps = prepared.planned_steps();
    if planned_steps.is_empty() {
        let _lock = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata,
            )
            .await?;
        return Ok(bulk::execute_prepared_bulk_toggle(
            &app,
            &state,
            &prepared,
            cancel_state.begin(),
        )
        .result);
    }

    let planned_sequences = prepared.planned_sequences();
    let journal_steps = planned_steps
        .into_iter()
        .map(|(sequence, old_path, new_path)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, old_path, new_path)
        })
        .collect();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "bulk-toggle",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let mut execution =
        bulk::execute_prepared_bulk_toggle(&app, &state, &prepared, cancel_state.begin());

    for sequence in &planned_sequences {
        if execution.applied_sequences.contains(sequence) {
            mutation_lease.mark_step_applied(*sequence)?;
        } else {
            mutation_lease.mark_step_rolled_back(*sequence)?;
        }
    }

    if execution.applied_sequences.is_empty() {
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Ok(execution.result);
    }

    let reconcile_result = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        &app,
        pool.inner(),
        &game_id,
        &mutation_lease,
    )
    .await;

    match reconcile_result {
        Ok(reconcile_result) => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(Ok(reconcile_result));
            apply_committed_reconcile(&mut execution.result, settlement);
            Ok(execution.result)
        }
        Err(error) => {
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) =
                bulk::rollback_prepared_bulk_toggle(&state, &prepared, &execution.applied_sequences)
            {
                let combined = format!("{error}; bulk toggle rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for sequence in &execution.applied_sequences {
                mutation_lease.mark_step_rolled_back(*sequence)?;
            }
            if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &game_id,
                &mutation_lease,
            )
            .await
            {
                let combined =
                    format!("{error}; rollback projection failed: {rollback_reconcile_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            mutation_lease.finish_rollback()?;
            Err(error)
        }
    }
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_delete_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
) -> Result<bulk::BulkResult, AppError> {
    // Required, like `delete_mod`: it names the mods root the paths must sit
    // inside, and the game whose index rows may be pruned. Optional, it let a
    // caller skip containment entirely.
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    if validated.is_empty() {
        return Ok(bulk::BulkResult::new(Vec::new(), Vec::new()));
    }
    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let prepared = bulk::prepare_bulk_delete(&validated)?;
    let planned_sequences = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, _, _)| sequence)
        .collect::<Vec<_>>();
    let journal_steps = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, source, quarantine)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, source, quarantine)
        })
        .collect();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "bulk-delete",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let mut execution =
        bulk::execute_prepared_bulk_delete(&app, &state, &prepared, cancel_state.begin());
    for sequence in &planned_sequences {
        if execution.applied_sequences.contains(sequence) {
            mutation_lease.mark_step_applied(*sequence)?;
        } else {
            mutation_lease.mark_step_rolled_back(*sequence)?;
        }
    }
    if execution.applied_sequences.is_empty() {
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Ok(execution.result);
    }
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
                    "Bulk delete reconcile requires attention: {:?}",
                    reconcile.status
                )),
                Err(error) => error,
            };
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) =
                bulk::rollback_prepared_bulk_delete(&state, &prepared, &execution.applied_sequences)
            {
                let combined = format!("{error}; bulk delete rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for sequence in &execution.applied_sequences {
                mutation_lease.mark_step_rolled_back(*sequence)?;
            }
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
    execution
        .result
        .collection_impact
        .merge(reconcile.collection_reference_impact);
    if let Err(error) = bulk::finalize_prepared_bulk_delete(&prepared, &execution.applied_sequences)
    {
        execution.result.sync_warning = Some(
            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning {
                kind: crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::CleanupPending,
                message: error.to_string(),
            },
        );
    }
    Ok(execution.result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_update_info(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, AppError> {
    if update.is_safe.is_some() {
        return Err(AppError::Validation(
            "Safety changes must use bulk_set_mod_safety".to_string(),
        ));
    }
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result = bulk::bulk_update_info(&validated, update).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_set_mod_safety(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    safe: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let resolved = bulk::resolve_safety_targets(pool.inner(), &game_id, &validated).await?;
    let suppression = watcher.suppressor.suppress_paths(
        resolved
            .targets
            .iter()
            .map(|target| std::path::Path::new(&target.disk_path)),
    );
    let mut result = bulk::bulk_set_safety(pool.inner(), &game_id, resolved, safe).await?;
    drop(suppression);
    drop(lock);

    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_toggle_favorite(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result =
        bulk::bulk_toggle_favorite(&pool, game_id.clone(), folder_paths, favorite).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_pin_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result = bulk::bulk_pin(&pool, game_id.clone(), folder_paths, pin).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_cancel_state_is_observed_and_reset_for_the_next_batch() {
        let state = BulkCancelState::new();

        assert!(!state.begin().load(Ordering::SeqCst));
        state.cancel();
        assert!(state.0.load(Ordering::SeqCst));
        assert!(!state.begin().load(Ordering::SeqCst));
    }
}
