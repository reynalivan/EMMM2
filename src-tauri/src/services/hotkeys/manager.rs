//! HotkeyManager — bridges OS-level global hotkeys to action planners.
//!
//! Owns global shortcut registration lifecycle through `tauri-plugin-global-shortcut`,
//! and dispatches events to action planners.
//!
//! **Threading model:**
//! - Registration/unregistration happens through Tauri plugin APIs.
//! - Event listening is callback-driven via plugin handler (configured in `lib.rs`).
//! - `HotkeyState` (debounce/switch_lock) is protected by `Mutex`.

use crate::domain::errors::AppError;
use std::collections::HashMap;
use std::sync::Mutex;

use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::common::sync::lock;
use crate::services::config::ConfigService;

use super::actions::{self, ActionResult, CycleDirection};
use super::cycle_preset::execute_cycle_preset;
use super::focus;
use super::{get_key_string, HotkeyAction, HotkeyConfig, HotkeyState};

// ─── Key Parsing ─────────────────────────────────────────────────────────────

/// Parse and normalize a user-facing key string (e.g. "F5", "Shift+F6").
pub fn parse_hotkey(key_str: &str) -> Result<String, AppError> {
    let normalized = normalize_shortcut(key_str);
    if normalized.is_empty() {
        return Err(AppError::Internal("Hotkey cannot be empty".to_string()));
    }

    // `normalize_shortcut` already stripped every space, so a token is empty
    // only when the string is all separators ("+", "++", …).
    if normalized.split('+').all(str::is_empty) {
        return Err(AppError::Internal(format!("Invalid hotkey '{key_str}'")));
    }

    Ok(normalized)
}

/// The one spelling of "how a shortcut string is canonicalized". Registration
/// and keystroke replay (`reload.rs`) must agree, or we register a shortcut we
/// cannot send.
pub fn normalize_shortcut(key_str: &str) -> String {
    key_str.trim().replace(' ', "").to_ascii_lowercase()
}

/// `Some` for the two preset-cycling actions, `None` for everything else.
fn preset_cycle_direction(action: HotkeyAction) -> Option<CycleDirection> {
    match action {
        HotkeyAction::NextPreset => Some(CycleDirection::Next),
        HotkeyAction::PrevPreset => Some(CycleDirection::Previous),
        _ => None,
    }
}

/// Why an action produced no backend work, for the status line.
fn noop_reason(action: HotkeyAction) -> &'static str {
    match action {
        HotkeyAction::ToggleOverlay => "Overlay toggle (handled by 3DMigoto)",
        _ => "Variant cycle triggered",
    }
}

// ─── Registration Map ────────────────────────────────────────────────────────

type HotkeyMap = HashMap<String, HotkeyAction>;

/// Build a map of (shortcut string, HotkeyAction) from the user config.
fn build_registration(config: &HotkeyConfig) -> Result<Vec<(String, HotkeyAction)>, AppError> {
    HotkeyAction::ALL
        .into_iter()
        .map(|action| Ok((parse_hotkey(get_key_string(config, action))?, action)))
        .collect()
}

// ─── HotkeyManager ──────────────────────────────────────────────────────────

/// Managed Tauri state — owns OS hotkey lifecycle.
pub struct HotkeyManager {
    /// Map from normalized shortcut string → action enum. Non-empty exactly
    /// while shortcuts are registered, so it also answers `is_enabled`.
    key_map: Mutex<HotkeyMap>,
    /// Debounce / switch-lock state.
    state: Mutex<HotkeyState>,
}

impl HotkeyManager {
    /// Create a new HotkeyManager.
    pub fn new(config: &HotkeyConfig) -> Self {
        Self {
            key_map: Mutex::new(HashMap::new()),
            state: Mutex::new(HotkeyState::new(config.cooldown_ms)),
        }
    }

    /// Register all shortcuts from the config with Tauri global shortcut plugin.
    fn register_all(&self, app: &tauri::AppHandle, config: &HotkeyConfig) -> Result<(), AppError> {
        let global_shortcut = app.global_shortcut();
        let entries = build_registration(config)?;
        let mut key_map = HashMap::new();

        global_shortcut.unregister_all()?;

        for (shortcut, action) in &entries {
            global_shortcut.register(shortcut.as_str())?;
            key_map.insert(shortcut.clone(), *action);
        }

        *lock(&self.key_map) = key_map;

        log::info!("Registered {} global shortcuts", entries.len());

        Ok(())
    }

    /// Unregister all shortcuts from the plugin.
    fn unregister_all(&self, app: &tauri::AppHandle) -> Result<(), AppError> {
        let global_shortcut = app.global_shortcut();
        global_shortcut.unregister_all()?;

        lock(&self.key_map).clear();

        log::info!("Unregistered all global shortcuts");

        Ok(())
    }

    /// Update shortcuts after settings change.
    /// Unregisters old shortcuts and registers new ones.
    pub fn update_bindings(
        &self,
        app: &tauri::AppHandle,
        config: &HotkeyConfig,
    ) -> Result<(), AppError> {
        self.unregister_all(app)?;

        if config.enabled {
            self.register_all(app, config)?;
        }

        // Update cooldown
        lock(&self.state).update_cooldown(config.cooldown_ms);

        Ok(())
    }

    /// Check if the manager is currently enabled and listening.
    pub fn is_enabled(&self) -> bool {
        !lock(&self.key_map).is_empty()
    }

    #[cfg(test)]
    pub fn set_enabled_for_test(&self, enabled: bool) {
        let mut key_map = lock(&self.key_map);
        key_map.clear();
        if enabled {
            key_map.insert("f6".to_string(), HotkeyAction::NextPreset);
        }
    }

    /// Look up which action corresponds to a shortcut string.
    pub fn lookup_action(&self, shortcut: &str) -> Option<HotkeyAction> {
        lock(&self.key_map)
            .get(&normalize_shortcut(shortcut))
            .copied()
    }

    /// Try to acquire the action lock (debounce + switch_lock).
    pub fn try_acquire(&self) -> bool {
        lock(&self.state).try_acquire()
    }

    /// Release the action lock after an action completes.
    pub fn release(&self) {
        lock(&self.state).release();
    }

    /// Called by plugin event handler when a shortcut is pressed.
    pub fn on_shortcut_pressed(&self, app: &tauri::AppHandle, shortcut: &str) {
        if !self.is_enabled() {
            return;
        }

        let action = match self.lookup_action(shortcut) {
            Some(action) => action,
            None => return,
        };

        let Some(config_state) = app.try_state::<ConfigService>() else {
            log::warn!("Hotkey ignored: ConfigService is unavailable");
            return;
        };

        let settings = config_state.get_settings();
        if !settings.hotkeys.enabled {
            return;
        }

        if !focus::is_active_game_focused(&settings) {
            return;
        }
        if let Some(direction) = preset_cycle_direction(action) {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                match execute_cycle_preset(&app_handle, direction).await {
                    Ok(summary) => log::info!("Hotkey {:?} → {}", action, summary),
                    Err(e) => log::error!("Preset cycle hotkey {:?} failed: {e}", action),
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });

            return;
        }

        let safe_mode = settings.safe_mode.enabled;
        if let Some(result) = self.dispatch_action(action, safe_mode, None, &[]) {
            log::info!("Hotkey {:?} → {}", action, result.summary);
        }
    }

    /// Dispatch a hotkey action to the appropriate planner.
    ///
    /// Returns `Some(ActionResult)` if the action was handled, or `None` if ignored.
    pub fn dispatch_action(
        &self,
        action: HotkeyAction,
        safe_mode: bool,
        current_preset: Option<&str>,
        available_presets: &[String],
    ) -> Option<ActionResult> {
        if !self.is_enabled() {
            return None;
        }

        if !self.try_acquire() {
            log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
            return None;
        }

        let result = match preset_cycle_direction(action) {
            Some(direction) => {
                match actions::resolve_next_preset(available_presets, current_preset, direction) {
                    Some(target) => actions::plan_cycle_preset(&target, safe_mode),
                    None => actions::plan_noop(action, "No presets available", safe_mode),
                }
            }
            // Overlay toggle is handled directly by 3DMigoto INI, and variant
            // cycling has no backend executor yet — both only report status.
            None => actions::plan_noop(action, noop_reason(action), safe_mode),
        };

        self.release();

        Some(result)
    }
}
