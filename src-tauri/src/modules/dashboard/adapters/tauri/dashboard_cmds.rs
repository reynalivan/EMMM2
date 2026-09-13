use crate::modules::system::application::app::dashboard::{
    self, ActiveKeyBinding, DashboardPayload,
};
use crate::shared::errors::AppError;
use tauri::{Manager, State};

/// Fetch all dashboard data in a single command for minimal IPC overhead.
///
#[specta::specta]
#[tauri::command]
pub async fn get_dashboard_stats(
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<DashboardPayload, AppError> {
    dashboard::get_dashboard_payload(pool.inner()).await
}

#[tauri::command]
#[specta::specta]
pub fn get_storage_size_backfill_status(
    backfill: State<
        '_,
        crate::modules::dashboard::application::storage_backfill::StorageSizeBackfillState,
    >,
) -> crate::modules::dashboard::application::storage_backfill::StorageSizeBackfillStatus {
    backfill.status()
}

/// Starts the one-time low-priority size backfill. The command returns before
/// filesystem work begins so the Dashboard remains responsive.
#[tauri::command]
#[specta::specta]
pub async fn start_storage_size_backfill(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    backfill: State<
        '_,
        crate::modules::dashboard::application::storage_backfill::StorageSizeBackfillState,
    >,
) -> Result<
    crate::modules::dashboard::application::storage_backfill::StorageSizeBackfillStatus,
    AppError,
> {
    use crate::modules::dashboard::application::storage_backfill;

    let game_ids = config
        .get_settings()
        .games
        .into_iter()
        .map(|game| game.id)
        .collect::<Vec<_>>();
    if storage_backfill::is_completed(pool.inner()).await? {
        return Ok(backfill.mark_completed_from_marker(game_ids.len()));
    }
    let Some(pending_game_ids) = backfill.begin(&game_ids) else {
        return Ok(backfill.status());
    };

    let pool = pool.inner().clone();
    let app_for_job = app.clone();
    tokio::spawn(async move {
        for game_id in pending_game_ids {
            let config =
                app_for_job.state::<crate::modules::settings::application::config::ConfigService>();
            let disk_reconcile_state = app_for_job.state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >();
            let watcher = app_for_job
                .state::<crate::modules::workspace::application::scanner::watcher::WatcherState>(
            );
            let operation_lock =
                app_for_job.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
            let result = crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                    pool: &pool,
                    config: config.inner(),
                    state: disk_reconcile_state.inner(),
                    watcher_suppressor: watcher.suppressor.clone(),
                    operation_lock: operation_lock.inner_lock(),
                    progress_reporter: None,
                },
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                    game_id.clone(),
                    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::StorageSizeBackfill,
                    Vec::new(),
                    true,
                ),
            )
            .await;

            let backfill = app_for_job
                .state::<crate::modules::dashboard::application::storage_backfill::StorageSizeBackfillState>();
            match result {
                Ok(result) if result.status.applied() => backfill.record_game_completed(&game_id),
                Ok(result) => {
                    backfill.fail(format!(
                        "Could not size '{}': {}",
                        game_id,
                        result
                            .error_message
                            .unwrap_or_else(|| format!("{:?}", result.status))
                    ));
                    return;
                }
                Err(error) => {
                    backfill.fail(format!("Could not size '{game_id}': {error}"));
                    return;
                }
            }
        }

        let backfill = app_for_job
            .state::<crate::modules::dashboard::application::storage_backfill::StorageSizeBackfillState>();
        if let Err(error) = storage_backfill::mark_completed(&pool).await {
            backfill.fail(format!(
                "Could not save storage backfill completion: {error}"
            ));
            return;
        }
        backfill.complete();
    });

    Ok(backfill.status())
}

/// Scan all enabled mods for a game and return their keybindings.
///
/// This is a filesystem-heavy operation (reads INI files from disk),
/// so it's a separate command from the main dashboard payload.
#[specta::specta]
#[tauri::command]
pub async fn get_active_keybindings(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<ActiveKeyBinding>, AppError> {
    dashboard::get_active_keybindings_service(pool.inner(), &game_id).await
}

#[cfg(test)]
#[path = "tests/dashboard_cmds_tests.rs"]
mod tests;
