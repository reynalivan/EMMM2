use crate::database::models::ConfigStatus;
use tauri_plugin_store::StoreExt;

/// Checks whether the app has a valid config with games configured.
/// Returns the config status to determine which screen to show.
///
/// - `FreshInstall`: No config or empty games list → show Welcome Screen
/// - `HasConfig`: Valid config with games → skip to Dashboard
/// - `CorruptConfig`: Config exists but malformed → reset + show Welcome
#[tauri::command]
pub async fn check_config_status(app: tauri::AppHandle) -> Result<ConfigStatus, String> {
    let store = app
        .store("config.json")
        .map_err(|e| format!("Failed to load config store: {e}"))?;

    // Check if games array exists and is non-empty
    match store.get("games") {
        Some(games_val) => {
            if let Some(arr) = games_val.as_array() {
                if arr.is_empty() {
                    Ok(ConfigStatus::FreshInstall)
                } else {
                    Ok(ConfigStatus::HasConfig)
                }
            } else {
                // games key exists but is not an array → corrupt
                log::warn!("Config 'games' key is not an array, resetting");
                store.clear();
                Ok(ConfigStatus::CorruptConfig)
            }
        }
        None => Ok(ConfigStatus::FreshInstall),
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

    // open::that(log_dir).map_err(|e| e.to_string())?; // Assuming open crate is available
    // Better to use tauri's shell open if possible, or std::process::Command for "explorer"
    // But since "open" crate is standard for cross-platform, lets use "open" crate if available.
    // If not, use standard Command "explorer" (windows only).
    // The user rules say "The USER's OS version is windows."

    std::process::Command::new("explorer")
        .arg(log_dir)
        .spawn()
        .map_err(|e| format!("Failed to open log folder: {}", e))?;

    Ok(())
}
