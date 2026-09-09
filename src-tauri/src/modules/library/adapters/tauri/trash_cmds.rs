use crate::modules::library::application::mods::trash;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;
use tauri::{AppHandle, Manager};

#[specta::specta]
#[tauri::command]
pub async fn delete_mod(
    app: AppHandle,
    path: String,
    game_id: String,
) -> Result<trash::DeleteModResult, AppError> {
    let config = app.state::<ConfigService>();
    let pool = app.state::<sqlx::SqlitePool>();
    let state = app.state::<WatcherState>();
    let disk_reconcile = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let op_lock = app.state::<MutationCoordinator>();

    // `game_id` is required: it names the mods root the path must sit inside.
    // Without it the delete used to skip containment entirely and trash any
    // absolute path the caller sent.
    let validated = validate_path(&config, &game_id, &path)?;
    let preflight_paths = [validated.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;

    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let prepared = trash::prepare_trash_move(validated.as_ref())?;
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "delete-mod",
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
                    "Delete reconcile requires attention: {:?}",
                    reconcile.status
                )),
                Err(error) => error,
            };
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) = prepared.rollback(&state) {
                let combined = format!("{error}; delete rollback failed: {rollback_error}");
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

    let mut result = trash::DeleteModResult {
        collection_impact: reconcile.collection_reference_impact,
        sync_warning: None,
    };
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

#[specta::specta]
#[tauri::command]
pub async fn open_recycle_bin() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg("shell:RecycleBinFolder")
            .spawn()
            .map_err(|error| AppError::Io(format!("Failed to open Recycle Bin: {error}")))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::Validation(
            "Opening the system Recycle Bin is only supported on Windows".to_string(),
        ))
    }
}

#[cfg(test)]
#[path = "tests/trash_cmds_tests.rs"]
mod tests;
