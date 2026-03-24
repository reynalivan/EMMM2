//! Tauri commands for hotkey management — bindings, conflicts, and config updates.

use crate::services::config::ConfigService;
use crate::services::hotkeys::manager::HotkeyManager;
use crate::services::hotkeys::{detect_conflicts, list_bindings, HotkeyAction};
use crate::services::scanner::watcher::WatcherState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// A single hotkey binding for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct HotkeyBinding {
    pub action: String,
    pub key: String,
}

/// A conflict between two hotkey actions sharing the same key.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct HotkeyConflict {
    pub action_a: String,
    pub action_b: String,
    pub key: String,
}

fn action_label(action: HotkeyAction) -> String {
    match action {
        HotkeyAction::ToggleSafeMode => "Toggle Safe Mode".to_string(),
        HotkeyAction::NextPreset => "Next Preset".to_string(),
        HotkeyAction::PrevPreset => "Previous Preset".to_string(),

        HotkeyAction::ToggleOverlay => "Toggle Overlay".to_string(),
        HotkeyAction::NextVariantFolder => "Next Variant".to_string(),
        HotkeyAction::PrevVariantFolder => "Previous Variant".to_string(),
    }
}

/// Get all current hotkey bindings for display in the settings UI.
#[specta::specta]
#[tauri::command]
pub async fn get_hotkey_bindings(
    config_state: State<'_, ConfigService>,
) -> Result<Vec<HotkeyBinding>, String> {
    let settings = config_state.get_settings();
    let bindings = list_bindings(&settings.hotkeys);

    Ok(bindings
        .into_iter()
        .map(|(action, key)| HotkeyBinding {
            action: action_label(action),
            key,
        })
        .collect())
}

/// Detect key conflicts in the current hotkey configuration.
#[specta::specta]
#[tauri::command]
pub async fn detect_hotkey_conflicts(
    config_state: State<'_, ConfigService>,
) -> Result<Vec<HotkeyConflict>, String> {
    let settings = config_state.get_settings();
    let conflicts = detect_conflicts(&settings.hotkeys);

    Ok(conflicts
        .into_iter()
        .map(|(a, b, key)| HotkeyConflict {
            action_a: action_label(a),
            action_b: action_label(b),
            key,
        })
        .collect())
}

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
