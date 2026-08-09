//! Tauri commands for hotkey management — bindings, conflicts, and config updates.

use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::hotkeys::manager::HotkeyManager;
use tauri::State;

/// Update hotkey config and re-register OS hotkeys.
/// This saves settings to DB AND tells the HotkeyManager to re-register.
#[specta::specta]
#[tauri::command]
pub async fn update_hotkey_config(
    app: tauri::AppHandle,
    config_state: State<'_, ConfigService>,
    hotkey_manager: State<'_, HotkeyManager>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(), AppError> {
    let settings = config_state.get_settings();
    hotkey_manager
        .inner()
        .update_bindings(&app, &settings.hotkeys)?;

    // Sync in-game overlay artifacts
    let _ = crate::services::app::post_apply::trigger_overlay_refresh(pool.inner(), &config_state)
        .await;

    Ok(())
}

/// The key 3DMigoto reloads its fixes on, read from the active game's
/// `d3dx.ini` (falling back to the loader default).
///
/// Toggling from the app moves folders on disk but cannot tell a running game
/// to re-read them: the reload keystroke has to be pressed while the game has
/// focus, so the UI names the key instead of replaying it into whatever window
/// happens to be in front. The in-game hotkey path replays it directly because
/// there the game IS focused (`services::hotkeys::reload`).
#[specta::specta]
#[tauri::command]
pub async fn get_reload_key(config_state: State<'_, ConfigService>) -> Result<String, AppError> {
    use crate::services::keyviewer::generator::{self, DEFAULT_RELOAD_KEY};

    Ok(config_state.with_settings(|settings| {
        settings
            .active_game()
            .and_then(|game| game.game_exe.parent().map(|root| root.join("d3dx.ini")))
            .map(|d3dx| generator::discover_reload_key(&d3dx).reload_fixes_key)
            .unwrap_or_else(|| DEFAULT_RELOAD_KEY.to_string())
    }))
}
