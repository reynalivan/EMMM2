//! File system watcher commands.

use crate::services::scanner::watcher::WatcherState;
use std::sync::atomic::Ordering;
use tauri::State;

/// Manually set watcher suppression state (e.g. for bulk operations).
///
/// # Covers: EC-2.06
#[tauri::command]
pub async fn set_watcher_suppression_cmd(
    suppressed: bool,
    watcher: State<'_, WatcherState>,
) -> Result<(), String> {
    watcher.suppressor.store(suppressed, Ordering::Relaxed);
    Ok(())
}

/// Start the file watcher for a specific path.
/// Emits `mod_watch:event` to the frontend.
///
/// Delegates the full lifecycle (thread spawning, event loop, DB sync) to
/// `services::scanner::watcher::lifecycle::start_watcher`.
#[tauri::command]
pub async fn start_watcher_cmd(
    app: tauri::AppHandle,
    path: String,
    game_id: String,
    state: State<'_, WatcherState>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(), String> {
    let db_pool = (*pool).clone();
    crate::services::scanner::watcher::lifecycle::start_watcher(app, &state, db_pool, path, game_id)
}

#[cfg(test)]
#[path = "tests/watcher_cmds_tests.rs"]
mod tests;
