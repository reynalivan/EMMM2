use crate::database::models::ConfigStatus;

/// Check if the app has any games configured (determines which screen to show on startup).
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, String> {
    crate::services::app::app_service::check_config_status(pool.inner()).await
}

/// Read the last N lines of the application log.
#[tauri::command]
pub async fn get_log_lines(app: tauri::AppHandle, lines: usize) -> Result<Vec<String>, String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_path = app_data_dir.join("logs").join("emmm2.log");
    crate::services::app::log_service::read_last_n_lines(&log_path, lines)
}

/// Open the logs directory in the OS file explorer.
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_dir = app_data_dir.join("logs");

    crate::services::app::log_service::open_log_folder_service(&log_dir)
}

/// Reset the application setup by clearing all data from the database.
/// Before clearing, a backup copy of `app.db` is saved to the trash folder.
/// No mod files or folders on disk are deleted — only database records are cleared.
#[tauri::command]
pub async fn reset_database(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<(), String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::services::app::app_service::reset_database_service(pool.inner(), &app_data_dir).await
}

#[cfg(test)]
#[path = "tests/app_cmds_tests.rs"]
mod tests;
