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
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let try_init = || async {
            let opts = SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true);

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
                try_init()
                    .await
                    .expect("Failed to initialize database after recovery")
            }
        };

        if let Err(e) = repo::stable_ids::migrate_to_stable_ids(&p).await {
            log::warn!("Stable ID migration skipped: {e}");
        }
        if let Err(e) = repo::unicode_keys::ensure_unicode_keys(&p).await {
            log::warn!("Unicode key backfill skipped: {e}");
        }
        p
    })
}

/// Builds the hotkey manager and registers its bindings. Falls back to a disabled
/// manager when the configured shortcuts cannot be registered; returns `None` only
/// when even the disabled fallback fails to construct.
pub fn init_hotkey_manager(
    app_handle: &tauri::AppHandle,
    hotkey_config: &services::hotkeys::HotkeyConfig,
) -> Option<services::hotkeys::manager::HotkeyManager> {
    match services::hotkeys::manager::HotkeyManager::new(hotkey_config) {
        Ok(hk_manager) => {
            let _ = hk_manager.update_bindings(app_handle, hotkey_config);
            Some(hk_manager)
        }
        Err(_) => {
            let disabled_config = services::hotkeys::HotkeyConfig {
                enabled: false,
                ..Default::default()
            };
            services::hotkeys::manager::HotkeyManager::new(&disabled_config).ok()
        }
    }
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
pub fn run_startup_reconcile(
    pool: &sqlx::SqlitePool,
    config: &services::config::ConfigService,
    watcher_state: &services::scanner::watcher::WatcherState,
    disk_reconcile_state: &services::disk_reconcile::orchestrator::DiskReconcileState,
) {
    use tauri::async_runtime::block_on;

    let settings = config.get_settings();
    block_on(async {
        match repo::task_repo::purge_old_tasks(pool).await {
            Ok(purged) if purged > 0 => {
                log::info!("startup: purged {purged} old task log(s) before boot reconcile");
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!("startup: task GC failed before boot reconcile: {error}");
            }
        }

        recover_interrupted_transfers(pool).await;

        let Some(active_game_id) = settings.active_game_id.as_deref() else {
            return;
        };
        let Some(game) = settings
            .games
            .iter()
            .find(|entry| entry.id == active_game_id)
        else {
            return;
        };
        let mod_path = game.mod_path.to_string_lossy().to_string();
        if mod_path.is_empty() {
            return;
        }

        match services::disk_reconcile::orchestrator::reconcile_disk_state(
            services::disk_reconcile::orchestrator::DiskReconcileContext {
                pool,
                config,
                state: disk_reconcile_state,
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
            Ok(_) => {}
            Err(error) => {
                log::warn!(
                    "Startup Disk Reconcile failed for '{}': {}",
                    game.name,
                    error
                );
            }
        }
    });
}
