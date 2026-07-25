use crate::domain::errors::AppError;
use crate::services::app::dashboard::{self, ActiveKeyBinding, DashboardPayload};

/// Fetch all dashboard data in a single command for minimal IPC overhead.
///
/// `safe_mode`: when true, dashboard stats/charts exclude mods with `is_safe = 0`.
#[specta::specta]
#[tauri::command]
pub async fn get_dashboard_stats(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    safe_mode: bool,
) -> Result<DashboardPayload, AppError> {
    dashboard::get_dashboard_payload(pool.inner(), safe_mode)
        .await
        .map_err(AppError::Internal)
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
    dashboard::get_active_keybindings_service(pool.inner(), &game_id)
        .await
        .map_err(AppError::Internal)
}

#[cfg(test)]
#[path = "tests/dashboard_cmds_tests.rs"]
mod tests;
