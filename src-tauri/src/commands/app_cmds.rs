use crate::database::models::ConfigStatus;
use tauri_plugin_store::StoreExt;
use log;

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
