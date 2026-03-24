use crate::database::models::ConfigStatus;

/// Check if the app has any games configured (determines which screen to show on startup).
#[specta::specta]
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, String> {
    crate::services::app::app_service::check_config_status(pool.inner()).await
}

/// Read the last N lines of the application log.
#[specta::specta]
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    limit: Option<usize>,
    count: Option<usize>,
) -> Result<Vec<String>, String> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    let log_path = log_dir.join("emmm.log");

    let lines = limit.or(count).unwrap_or(200);
    crate::services::app::log_service::read_last_n_lines(&log_path, lines)
}

/// Open the logs directory in the OS file explorer.
#[specta::specta]
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;

    crate::services::app::log_service::open_log_folder_service(&log_dir)
}

/// Reset the application setup by clearing all data from the database.
/// Before clearing, a backup copy of `app.db` is saved to the trash folder.
/// No mod files or folders on disk are deleted — only database records are cleared.
#[specta::specta]
#[tauri::command]
pub async fn reset_database(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, crate::services::config::ConfigService>,
) -> Result<(), String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    crate::services::app::app_service::reset_database_service(pool.inner(), &app_data_dir).await?;

    // Clear out the in-memory singleton state
    config.reset_to_default();

    Ok(())
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
