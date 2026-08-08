//! Startup bootstrap steps extracted from the Tauri `.setup()` closure.
//!
//! Ordering is load-bearing: window recovery -> thumbnail cache -> database pool
//! (with corrupt-db recovery) -> config -> hotkeys -> task GC + boot reconcile.
//! `lib.rs` keeps the `.manage()` / plugin / command registration and calls these
//! in the same order.

use tauri::Manager;

use crate::repo;
use crate::services;

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

        if let Err(e) = repo::unicode_keys::ensure_unicode_keys(&p).await {
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
    hotkey_config: &services::hotkeys::HotkeyConfig,
) -> services::hotkeys::manager::HotkeyManager {
    let hk_manager = services::hotkeys::manager::HotkeyManager::new(hotkey_config);
    if let Err(error) = hk_manager.update_bindings(app_handle, hotkey_config) {
        log::warn!("startup: hotkey registration failed, continuing disabled: {error}");
    }
    hk_manager
}

/// Marks browser downloads and import jobs that were mid-flight when the process
/// last exited as `failed`. Neither a reqwest stream nor an import pipeline survives
/// a restart, so without this they stay `in_progress`/`extracting` forever and the
/// user can never retry them.
async fn recover_interrupted_transfers(pool: &sqlx::SqlitePool) {
    match repo::browser_repo::fail_interrupted_downloads(pool).await {
        Ok(count) if count > 0 => log::info!("startup: failed {count} interrupted download(s)"),
        Ok(_) => {}
        Err(error) => log::warn!("startup: download recovery failed: {error}"),
    }
    match repo::browser_repo::fail_interrupted_jobs(pool).await {
        Ok(count) if count > 0 => log::info!("startup: failed {count} interrupted import job(s)"),
        Ok(_) => {}
        Err(error) => log::warn!("startup: import job recovery failed: {error}"),
    }
}

/// Purges stale task rows, fails downloads and import jobs a crash left in
/// flight, then reconciles the active game's mod folder against the database.
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
        match repo::task_repo::purge_old_tasks(&pool).await {
            Ok(purged) if purged > 0 => {
                log::info!("startup: purged {purged} old task log(s) before boot reconcile");
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!("startup: task GC failed before boot reconcile: {error}");
            }
        }

        recover_interrupted_transfers(&pool).await;
    });

    spawn(async move {
        let config = app.state::<services::config::ConfigService>();
        let Some(game) = config.with_settings(|settings| settings.active_game().cloned()) else {
            return;
        };
        if game.mod_path.as_os_str().is_empty() {
            return;
        }

        let watcher_state = app.state::<services::scanner::watcher::WatcherState>();
        let disk_reconcile_state =
            app.state::<services::disk_reconcile::orchestrator::DiskReconcileState>();

        if let Err(error) = services::disk_reconcile::orchestrator::reconcile_disk_state(
            services::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &pool,
                config: config.inner(),
                state: disk_reconcile_state.inner(),
                watcher_suppressor: watcher_state.suppressor.clone(),
            },
            services::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                game.id.clone(),
                services::disk_reconcile::types::DiskReconcileReason::StartupBoot,
                Vec::new(),
                true,
            ),
        )
        .await
        {
            log::warn!(
                "Startup Disk Reconcile failed for '{}': {}",
                game.name,
                error
            );
        }
    });
}
