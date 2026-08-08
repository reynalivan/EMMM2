use crate::domain::errors::AppError;
use crate::services::app::dashboard::{self, ActiveKeyBinding, DashboardPayload};

/// Fetch all dashboard data in a single command for minimal IPC overhead.
///
/// The corridor is derived server-side; in Safe Mode the stats and charts
/// exclude mods with `is_safe = 0`.
#[specta::specta]
#[tauri::command]
pub async fn get_dashboard_stats(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::services::config::ConfigService>,
) -> Result<DashboardPayload, AppError> {
    dashboard::get_dashboard_payload(pool.inner(), config.current_corridor()).await
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
