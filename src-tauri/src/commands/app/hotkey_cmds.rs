//! Tauri commands for hotkey management — bindings, conflicts, and config updates.

use crate::services::config::ConfigService;
use crate::services::hotkeys::manager::HotkeyManager;
use crate::services::scanner::watcher::WatcherState;
use tauri::State;

/// Update hotkey config and re-register OS hotkeys.
/// This saves settings to DB AND tells the HotkeyManager to re-register.
#[specta::specta]
#[tauri::command]
pub async fn update_hotkey_config(
    app: tauri::AppHandle,
    config_state: State<'_, ConfigService>,
    hotkey_manager: State<'_, HotkeyManager>,
    watcher_state: State<'_, WatcherState>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(), String> {
    let settings = config_state.get_settings();
    hotkey_manager
        .inner()
        .update_bindings(&app, &settings.hotkeys)?;

    // Sync in-game overlay artifacts
    let _ = crate::services::app::post_apply::trigger_overlay_refresh(
        pool.inner(),
        &config_state,
        watcher_state.suppressor.clone(),
    )
    .await;

    Ok(())
}
