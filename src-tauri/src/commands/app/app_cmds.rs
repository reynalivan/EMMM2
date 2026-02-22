use crate::database::models::ConfigStatus;
use crate::database::settings_repo;

/// Checks whether the app has games configured in the database.
/// Returns the config status to determine which screen to show.
///
/// - `FreshInstall`: No games → show Welcome Screen
/// - `HasConfig`: Has games → skip to Dashboard
#[tauri::command]
pub async fn check_config_status(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<ConfigStatus, String> {
    let count = settings_repo::count_games(pool.inner())
        .await
        .map_err(|e| format!("Failed to check config status: {e}"))?;

    if count > 0 {
        Ok(ConfigStatus::HasConfig)
    } else {
        Ok(ConfigStatus::FreshInstall)
    }
}

/// Read the last N lines of the application log.
#[tauri::command]
pub async fn get_log_lines(app: tauri::AppHandle, lines: usize) -> Result<Vec<String>, String> {
    use std::io::BufRead;
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_path = app_data_dir.join("logs").join("emmm2.log");

    if !log_path.exists() {
        return Ok(vec!["Log file not found.".to_string()]);
    }

    let file = std::fs::File::open(&log_path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);

    let all_lines: Result<Vec<String>, _> = reader.lines().collect();
    let all_lines = all_lines.map_err(|e| e.to_string())?;

    let count = all_lines.len();
    let skip = count.saturating_sub(lines);

    Ok(all_lines.into_iter().skip(skip).collect())
}

/// Open the logs directory in the OS file explorer.
#[tauri::command]
pub async fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_dir = app_data_dir.join("logs");

    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    }

    std::process::Command::new("explorer")
        .arg(log_dir)
        .spawn()
        .map_err(|e| format!("Failed to open log folder: {}", e))?;

    Ok(())
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
    let db_path = app_data_dir.join("app.db");
    let trash_dir = app_data_dir.join("trash");

    // Ensure trash directory exists
    if !trash_dir.exists() {
        std::fs::create_dir_all(&trash_dir)
            .map_err(|e| format!("Failed to create trash dir: {e}"))?;
    }

    // Backup the database file with a timestamp
    if db_path.exists() {
        let epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_name = format!("app_backup_{}.db", epoch_secs);
        let backup_path = trash_dir.join(&backup_name);
        std::fs::copy(&db_path, &backup_path)
            .map_err(|e| format!("Failed to backup database: {e}"))?;
        log::info!("Database backed up to: {}", backup_path.display());
    }

    // Clear all data from the database (tables only, no file deletion)
    settings_repo::reset_all_data(pool.inner())
        .await
        .map_err(|e| format!("Failed to reset database: {e}"))
}
