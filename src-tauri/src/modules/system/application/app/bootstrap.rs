//! Startup bootstrap steps extracted from the Tauri `.setup()` closure.
//!
//! Ordering is load-bearing: window recovery -> thumbnail cache -> database pool
//! (with corrupt-db recovery) -> config -> hotkeys -> task GC + boot reconcile.
//! `lib.rs` keeps the `.manage()` / plugin / command registration and calls these
//! in the same order.

use tauri::{Emitter, Manager};

const MUTATION_JOURNAL_HISTORY_LIMIT: usize = 256;

async fn initialize_mutation_coordinator(
    app: &tauri::AppHandle,
) -> Result<Vec<String>, crate::shared::errors::AppError> {
    let app_data_dir = app.path().app_data_dir()?;
    let journal_path = app_data_dir.join("mutation-journal.json");
    let journal = std::sync::Arc::new(crate::modules::mutation::journal::OperationJournal::open(
        journal_path,
        MUTATION_JOURNAL_HISTORY_LIMIT,
    )?);
    let game_roots = app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .with_settings(|settings| {
            settings
                .games
                .iter()
                .map(|game| (game.id.clone(), game.mod_path.clone()))
                .collect::<std::collections::HashMap<_, _>>()
        });
    let roots = crate::modules::mutation::recovery::RecoveryRoots::new(
        game_roots.clone(),
        app_data_dir.join("import-staging"),
    );
    let recovered = crate::modules::mutation::recovery::RecoveryRunner::new(journal.clone(), roots)
        .run_recovery()
        .await?;
    let recorded_quarantines = journal
        .entries()
        .into_iter()
        .filter(|operation| {
            matches!(
                operation.kind.as_str(),
                "delete-mod" | "bulk-delete" | "trash-folder-conflict-candidate"
            ) && operation.status == crate::modules::mutation::journal::OperationStatus::Completed
                && operation.database_projection_status
                    == crate::modules::mutation::journal::DatabaseProjectionStatus::Committed
        })
        .flat_map(|operation| {
            let game_root = game_roots.get(&operation.game_id).cloned();
            operation.steps.into_iter().filter_map(move |step| {
                let path = (step.status == crate::modules::mutation::journal::StepStatus::Applied)
                    .then_some(step.new_path)
                    .flatten()?;
                game_root
                    .as_ref()
                    .is_some_and(|root| path.starts_with(root))
                    .then_some(path)
            })
        })
        .collect::<Vec<_>>();
    for warning in crate::modules::library::application::mods::trash::finalize_recorded_quarantines(
        recorded_quarantines,
    ) {
        log::warn!("Pending trash quarantine cleanup failed: {warning}");
    }
    app.state::<crate::modules::mutation::coordinator::MutationCoordinator>()
        .configure(journal)?;
    Ok(recovered)
}

/// Re-centers the main window when it was restored onto a monitor that no longer
/// exists (disconnected display).
pub fn center_window_if_offscreen(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Ok(outer_pos) = window.outer_position() {
            let mut is_visible = false;
            if let Ok(monitors) = window.available_monitors() {
                for monitor in monitors {
                    let m_pos = monitor.position();
                    let m_size = monitor.size();
                    if outer_pos.x >= m_pos.x
                        && outer_pos.x < m_pos.x + m_size.width as i32
                        && outer_pos.y >= m_pos.y
                        && outer_pos.y < m_pos.y + m_size.height as i32
                    {
                        is_visible = true;
                        break;
                    }
                }
            }
            if !is_visible {
                log::warn!(
                    "Window spawned off-screen (disconnected monitor). Centering on primary."
                );
                let _ = window.center();
            }
        }
    }
}

/// Opens the SQLite pool and runs migrations. A corrupt database is renamed to
/// `app_corrupt_<unix_ts>.db` and re-created from scratch. Stable-id and
/// unicode-key backfills run afterwards and are best-effort.
#[cfg(desktop)]
pub fn init_pool(app_data_dir: &std::path::Path) -> sqlx::SqlitePool {
    use tauri::async_runtime::block_on;

    let db_path = app_data_dir.join("app.db");
    if !app_data_dir.exists() {
        let _ = std::fs::create_dir_all(app_data_dir);
    }

    block_on(async {
        use sqlx::sqlite::{
            SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
        };
        let try_init = || async {
            // sqlx leaves these unset, so SQLite falls back to `DELETE` + `FULL`
            // — roughly three fsyncs per autocommit statement, paid per row by
            // every bulk toggle, delete and projection refresh.
            //
            // `NORMAL` under WAL can lose the last transactions on power loss,
            // which is the right trade here: the filesystem is the source of
            // truth and a lost index write is repaired by the next reconcile.
            let opts = SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal);

            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(opts)
                .await?;

            sqlx::migrate!("./migrations").run(&pool).await?;

            Ok::<sqlx::SqlitePool, sqlx::Error>(pool)
        };

        let p = match try_init().await {
            Ok(pool) => pool,
            Err(e) => {
                log::error!("Database connection or migration failed: {e}. Attempting recovery...");
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let backup_path = app_data_dir.join(format!("app_corrupt_{}.db", timestamp));
                let _ = std::fs::rename(&db_path, &backup_path);
                // WAL keeps its state in sidecar files; leaving them behind
                // would hand the freshly created database a stale journal.
                for suffix in ["-wal", "-shm"] {
                    let sidecar = app_data_dir.join(format!("app.db{suffix}"));
                    let _ = std::fs::remove_file(sidecar);
                }
                try_init()
                    .await
                    .expect("Failed to initialize database after recovery")
            }
        };

        if let Err(e) =
            crate::modules::system::adapters::sqlite::utils::unicode_keys::ensure_unicode_keys(&p)
                .await
        {
            log::warn!("Unicode key backfill skipped: {e}");
        }
        p
    })
}

/// Builds the hotkey manager and registers its bindings. Construction cannot
/// fail; a shortcut the OS refuses just leaves the manager with no bindings
/// registered, which reads as disabled.
pub fn init_hotkey_manager(
    app_handle: &tauri::AppHandle,
    hotkey_config: &crate::modules::automation::application::hotkeys::HotkeyConfig,
) -> crate::modules::automation::application::hotkeys::manager::HotkeyManager {
    let hk_manager = crate::modules::automation::application::hotkeys::manager::HotkeyManager::new(
        hotkey_config,
    );
    if let Err(error) = hk_manager.update_bindings(app_handle, hotkey_config) {
        log::warn!("startup: hotkey registration failed, continuing disabled: {error}");
    }
    hk_manager
}

/// Marks browser downloads that were mid-flight when the process last exited as
/// `failed`. The import-batch recovery below owns resumable import state.
async fn recover_interrupted_transfers(pool: &sqlx::SqlitePool) {
    match crate::modules::browser::adapters::sqlite::browser::fail_interrupted_downloads(pool).await
    {
        Ok(count) if count > 0 => log::info!("startup: failed {count} interrupted download(s)"),
        Ok(_) => {}
        Err(error) => log::warn!("startup: download recovery failed: {error}"),
    }
}

/// Purges stale task rows, fails downloads a crash left in flight, then
/// reconciles the active game's mod folder against the database.
/// Every step is best-effort and only logs on failure.
/// Boot-time database housekeeping, plus a background reconcile.
///
/// The two recovery writes stay on the setup thread: they are single UPDATEs,
/// and the frontend must not open onto a stale crash-recovery queue. The
/// reconcile does not — it walks the entire mods folder, and `.setup()` has to
/// return before the window appears. It reports through the progress events it
/// already emits, so the UI shows it running instead of showing nothing.
pub fn run_startup_reconcile(app: tauri::AppHandle) {
    use tauri::async_runtime::{block_on, spawn};
    use tauri::Manager;

    let pool = app.state::<sqlx::SqlitePool>().inner().clone();

    block_on(async {
        match initialize_mutation_coordinator(&app).await {
            Ok(recovered) if !recovered.is_empty() => {
                log::warn!(
                    "startup: recovered or isolated {} interrupted mutation(s): {}",
                    recovered.len(),
                    recovered.join(", ")
                );
            }
            Ok(_) => {}
            Err(error) => {
                log::error!("startup: mutation coordinator initialization failed: {error}")
            }
        }

        match crate::modules::workspace::adapters::sqlite::task::reclaim_interrupted_apply_tasks(
            &pool,
        )
        .await
        {
            Ok(reclaimed) if reclaimed > 0 => {
                log::info!("startup: reclaimed {reclaimed} interrupted collection apply task(s)");
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!("startup: interrupted collection apply reclaim failed: {error}");
            }
        }
        match crate::modules::workspace::adapters::sqlite::task::purge_old_tasks(&pool).await {
            Ok(purged) if purged > 0 => {
                log::info!("startup: purged {purged} old task log(s) before boot reconcile");
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!("startup: task GC failed before boot reconcile: {error}");
            }
        }

        recover_interrupted_transfers(&pool).await;
        match crate::modules::ingestion::adapters::sqlite::import_batch::recover_interrupted_batch_states(&pool).await {
            Ok(recovered) if recovered > 0 => {
                log::info!("startup: made {recovered} interrupted import state(s) resumable");
            }
            Ok(_) => {}
            Err(error) => log::warn!("startup: import batch recovery failed: {error}"),
        }
        match app.path().app_data_dir() {
            Ok(app_data) => {
                let staging_root = app_data.join("import-staging");
                match crate::modules::ingestion::adapters::sqlite::import_batch::list_terminal_batch_ids_for_staging_cleanup(&pool).await {
                    Ok(batch_ids) => {
                        for batch_id in batch_ids {
                            if let Err(error) = crate::modules::ingestion::application::import_batch::staging::cleanup_batch_staging(
                                &staging_root,
                                &batch_id,
                            ) {
                                log::warn!(
                                    "startup: batch staging cleanup failed for {batch_id}: {error}"
                                );
                            }
                        }
                    }
                    Err(error) => log::warn!("startup: terminal batch cleanup lookup failed: {error}"),
                }
                match crate::modules::ingestion::adapters::sqlite::import_batch::list_active_staging_paths(&pool).await {
                    Ok(paths) => match crate::modules::ingestion::application::import_batch::staging::cleanup_orphaned_staging(
                        &staging_root,
                        &paths,
                    ) {
                        Ok(removed) if removed > 0 => {
                            log::info!("startup: removed {removed} orphaned import staging attempt(s)");
                        }
                        Ok(_) => {}
                        Err(error) => log::warn!("startup: orphaned staging cleanup failed: {error}"),
                    },
                    Err(error) => log::warn!("startup: active staging lookup failed: {error}"),
                }
            }
            Err(error) => log::warn!("startup: app data path unavailable: {error}"),
        }
    });

    let startup_game = app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .with_settings(|settings| settings.active_game().cloned())
        .filter(|game| !game.mod_path.as_os_str().is_empty());
    let startup_recovery_generation = startup_game.as_ref().map(|game| {
        app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
            .mark_initial_recovery_pending(&game.id)
    });

    spawn(async move {
        let Some(game) = startup_game else {
            return;
        };
        let recovery_generation = startup_recovery_generation
            .expect("startup game and recovery generation are created together");
        let config = app.state::<crate::modules::settings::application::config::ConfigService>();
        let watcher_state =
            app.state::<crate::modules::workspace::application::scanner::watcher::WatcherState>();
        let disk_reconcile_state =
            app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let mutation_coordinator =
            app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();

        let reconcile_result = crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &pool,
                config: config.inner(),
                state: disk_reconcile_state.inner(),
                watcher_suppressor: watcher_state.suppressor.clone(),
                operation_lock: mutation_coordinator.inner_lock(),
                progress_reporter: Some(std::sync::Arc::new(
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                        app.clone(),
                        game.id.clone(),
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::StartupBoot,
                    ),
                )),
            },
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                game.id.clone(),
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::StartupBoot,
                Vec::new(),
                true,
            ),
        )
        .await;
        if config.get_settings().diagnostics.telemetry_enabled {
            let (outcome, error_code) = match &reconcile_result {
                Ok(_) => (
                    crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                    crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                ),
                Err(error) => (
                    crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
                    crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(error),
                ),
            };
            let telemetry = app
                .state::<crate::modules::system::application::telemetry::TelemetryStore>()
                .inner()
                .clone();
            let _ = telemetry
                .record_rollup(
                    env!("CARGO_PKG_VERSION"),
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
                        outcome,
                        error_code,
                    ),
                    chrono::Utc::now(),
                )
                .await;
        }
        let recovery_outcome = match reconcile_result {
            Ok(result) => {
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(Box::new(
                    result,
                ))
            }
            Err(error) => {
                log::warn!(
                    "Startup Disk Reconcile failed for '{}': {}",
                    game.name,
                    error
                );
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                    error.to_string(),
                )
            }
        };
        let completed_result = match &recovery_outcome {
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(result) => {
                Some((**result).clone())
            }
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(_) => None,
        };
        disk_reconcile_state.finish_initial_recovery(
            &game.id,
            recovery_generation,
            recovery_outcome,
        );
        if let Some(result) = completed_result {
            if let Err(error) = app.emit("disk_reconcile:result", result) {
                log::warn!("Could not emit startup disk reconcile result: {error}");
            }
        }
    });
}
