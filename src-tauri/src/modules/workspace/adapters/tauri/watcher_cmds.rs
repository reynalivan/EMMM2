//! File system watcher commands.

use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use tauri::{Manager, State};

/// Start the file watcher for a specific path.
/// Emits `mod_watch:event` to the frontend.
///
/// Delegates the full lifecycle (thread spawning, event loop, DB sync) to
/// `services::scanner::watcher::lifecycle::start_watcher`.
#[tauri::command]
#[specta::specta]
pub async fn start_watcher(
    app: tauri::AppHandle,
    path: String,
    game_id: String,
    state: State<'_, WatcherState>,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<(), AppError> {
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .inner()
        .clone();
    let started_at = std::time::Instant::now();
    let configured_root = crate::platform::fs::guard::validate_mods_root(&config, &game_id, &path)?;
    let db_pool = (*pool).clone();
    let result = crate::modules::workspace::application::scanner::watcher::lifecycle::start_watcher(
        app,
        &state,
        db_pool,
        configured_root.to_string_lossy().into_owned(),
        game_id,
    );
    if diagnostics_enabled && result.is_ok() {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::Watcher,
            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
        )
        .with_duration(started_at.elapsed());
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    result.map_err(Into::into)
}

/// Stop the file watcher. Cleanly drops the `RecommendedWatcher`,
/// terminating the background event loop thread.
///
/// Called by the frontend in `useEffect` cleanup when the active game changes
/// or the component unmounts.
///
/// # Covers: req-05 AC-05.2.2, req-28 (Game Switch → stop → init)
#[tauri::command]
#[specta::specta]
pub async fn stop_watcher(watcher: State<'_, WatcherState>) -> Result<(), AppError> {
    watcher.invalidate_session();
    let mut w = lock(&watcher.watcher);
    if w.is_some() {
        log::info!("Stopping watcher via command");
        *w = None;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/watcher_cmds_tests.rs"]
mod tests;
