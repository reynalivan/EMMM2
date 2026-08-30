use crate::shared::errors::AppError;
use crate::modules::games::domain::models::ConfigStatus;

/// Check if the app has any games configured (determines which screen to show on startup).
#[specta::specta]
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, AppError> {
    crate::modules::system::application::app::app_service::check_config_status(pool.inner()).await
}

/// Read the last N lines of the application log.
#[specta::specta]
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    limit: Option<usize>,
    count: Option<usize>,
) -> Result<Vec<String>, AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;
    let log_path = log_dir.join("emmm.log");

    let lines = limit.or(count).unwrap_or(200);
    crate::modules::system::application::app::log_service::read_last_n_lines(&log_path, lines)
}

/// Open the logs directory in the OS file explorer.
#[specta::specta]
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;

    crate::modules::system::application::app::log_service::open_log_folder_service(&log_dir)
}

/// Reset the application setup by clearing all data from the database.
/// Before clearing, a backup copy of `app.db` is saved to the trash folder.
/// No mod files or folders on disk are deleted — only database records are cleared.
#[specta::specta]
#[tauri::command]
pub async fn reset_database(
    app: tauri::AppHandle,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<(), AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;

    config.reset_database(&app_data_dir)
}

/// Check if a given absolute path exists on the disk.
/// Bypasses restrictive Tauri v2 plugin-fs scopes.
#[specta::specta]
#[tauri::command]
pub fn check_path_exists_cmd(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[cfg(test)]
#[path = "tests/app_cmds_tests.rs"]
mod tests;
