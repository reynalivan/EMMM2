use crate::domain::errors::AppError;
use crate::domain::models::ConfigStatus;

/// Check if the app has any games configured (determines which screen to show on startup).
#[specta::specta]
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, AppError> {
    crate::services::app::app_service::check_config_status(pool.inner())
        .await
        .map_err(AppError::Internal)
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
    crate::services::app::log_service::read_last_n_lines(&log_path, lines)
        .map_err(AppError::Internal)
}

/// Open the logs directory in the OS file explorer.
#[specta::specta]
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir()?;

    crate::services::app::log_service::open_log_folder_service(&log_dir).map_err(AppError::Internal)
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
) -> Result<(), AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;

    crate::services::app::app_service::reset_database_service(pool.inner(), &app_data_dir)
        .await
        .map_err(AppError::Internal)?;

    // Clear out the in-memory singleton state
    config.reset_to_default();

    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn close_splashscreen(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        main.show()?;
        main.set_focus()?;
    }
    Ok(())
}

/// Check if a given absolute path exists on the disk.
/// Bypasses restrictive Tauri v2 plugin-fs scopes.
#[specta::specta]
#[tauri::command]
pub fn check_path_exists_cmd(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

/// Ensure a directory exists on disk.
/// Used by frontend import flows to avoid direct plugin-fs dependency in tests/runtime.
#[specta::specta]
#[tauri::command]
pub fn ensure_dir_cmd(path: String) -> Result<(), AppError> {
    std::fs::create_dir_all(&path)
        .map_err(|e| AppError::Io(format!("Failed to create directory {path}: {e}")))
}

#[cfg(test)]
#[path = "tests/app_cmds_tests.rs"]
mod tests;
