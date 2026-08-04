//! File system watcher commands.

use crate::common::sync::lock;
use crate::domain::errors::AppError;
use crate::services::scanner::watcher::WatcherState;
use std::sync::atomic::Ordering;
use tauri::State;

/// Manually set watcher suppression state (e.g. for bulk operations).
///
/// # Covers: EC-2.06
#[tauri::command]
#[specta::specta]
pub async fn set_watcher_suppression(
    suppressed: bool,
    watcher: State<'_, WatcherState>,
) -> Result<(), AppError> {
    watcher.suppressor.store(suppressed, Ordering::Relaxed);
    Ok(())
}

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
) -> Result<(), AppError> {
    let db_pool = (*pool).clone();
    crate::services::scanner::watcher::lifecycle::start_watcher(app, &state, db_pool, path, game_id)
        .map_err(AppError::Internal)
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
