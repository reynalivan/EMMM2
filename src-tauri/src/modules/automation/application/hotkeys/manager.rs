//! HotkeyManager — bridges OS-level global hotkeys to action planners.
//!
//! Owns global shortcut registration lifecycle through `tauri-plugin-global-shortcut`,
//! and dispatches events to action planners.
//!
//! **Threading model:**
//! - Registration/unregistration happens through Tauri plugin APIs.
//! - Event listening is callback-driven via plugin handler (configured in `lib.rs`).
//! - `HotkeyState` (debounce/switch_lock) is protected by `Mutex`.

use crate::shared::errors::AppError;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::modules::settings::application::config::ConfigService;
use crate::shared::sync::lock;

use super::cycle_preset::{execute_cycle_preset, CycleDirection};
use super::focus;
use super::{get_key_string, HotkeyAction, HotkeyConfig, HotkeyState};

// ─── Key Parsing ─────────────────────────────────────────────────────────────

/// Parse and normalize a user-facing key string (e.g. "F5", "Shift+F6").
pub fn parse_hotkey(key_str: &str) -> Result<String, AppError> {
    let normalized = super::validate_3dmigoto_binding(key_str)?;
    if normalized.is_empty() {
        return Err(AppError::Internal("Hotkey cannot be empty".to_string()));
    }

    // `normalize_shortcut` already stripped every space, so a token is empty
    // only when the string is all separators ("+", "++", …).
    if normalized.split('+').all(str::is_empty) {
        return Err(AppError::Internal(format!("Invalid hotkey '{key_str}'")));
    }

    if normalized
        .split('+')
        .any(|token| matches!(token, "no_ctrl" | "no_shift" | "no_alt"))
    {
        return Err(AppError::Validation(
            "OS hotkeys cannot use NO_CTRL, NO_SHIFT, or NO_ALT modifiers".to_string(),
        ));
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

// ─── Registration Map ────────────────────────────────────────────────────────

type HotkeyMap = HashMap<String, HotkeyAction>;

/// Build a map of (shortcut string, HotkeyAction) from the user config.
fn build_registration(config: &HotkeyConfig) -> Result<Vec<(String, HotkeyAction)>, AppError> {
    let mut configured = HashSet::new();
    for action in HotkeyAction::ALL {
        let shortcut = super::validate_3dmigoto_binding(get_key_string(config, action))?;
        if !configured.insert(shortcut) {
            return Err(AppError::Validation(
                "Hotkey bindings must not use the same shortcut".to_string(),
            ));
        }
    }

    let entries: Vec<(String, HotkeyAction)> = HotkeyAction::OS_ACTIONS
        .into_iter()
        .map(|action| Ok((parse_hotkey(get_key_string(config, action))?, action)))
        .collect::<Result<_, AppError>>()?;
    Ok(entries)
}

// ─── HotkeyManager ──────────────────────────────────────────────────────────

/// Managed Tauri state — owns OS hotkey lifecycle.
pub struct HotkeyManager {
    /// Map from normalized shortcut string → action enum. Non-empty exactly
    /// while shortcuts are registered, so it also answers `is_enabled`.
    key_map: Mutex<HotkeyMap>,
    /// Debounce / switch-lock state.
    state: Mutex<HotkeyState>,
    /// One-shot release signals for OS shortcuts that will later synthesize a
    /// 3DMigoto reload chord. The signal is armed before the async work starts
    /// so a quick key release cannot be lost.
    release_waiters: Mutex<HashMap<String, Arc<tokio::sync::Notify>>>,
}

impl HotkeyManager {
    /// Create a new HotkeyManager.
    pub fn new(_config: &HotkeyConfig) -> Self {
        Self {
            key_map: Mutex::new(HashMap::new()),
            state: Mutex::new(HotkeyState::new()),
            release_waiters: Mutex::new(HashMap::new()),
        }
    }

    /// Update shortcuts after settings change, restoring the old registration
    /// if the OS rejects any part of the replacement set.
    pub fn update_bindings(
        &self,
        app: &tauri::AppHandle,
        config: &HotkeyConfig,
    ) -> Result<(), AppError> {
        validate_binding_configuration(config)?;
        let entries = config
            .enabled
            .then(|| build_registration(config))
            .transpose()?;
        let previous: Vec<(String, HotkeyAction)> = lock(&self.key_map)
            .iter()
            .map(|(shortcut, action)| (shortcut.clone(), *action))
            .collect();
        let global_shortcut = app.global_shortcut();
        global_shortcut.unregister_all()?;

        let registration = entries.as_deref().unwrap_or_default();
        let mut registered = HashMap::new();
        for (shortcut, action) in registration {
            if let Err(error) = global_shortcut.register(shortcut.as_str()) {
                let _ = global_shortcut.unregister_all();
                let mut restored = HashMap::new();
                for (old_shortcut, old_action) in &previous {
                    if let Err(restore_error) = global_shortcut.register(old_shortcut.as_str()) {
                        log::error!(
                            "Could not restore global hotkey '{old_shortcut}' after failed update: {restore_error}"
                        );
                        continue;
                    }
                    restored.insert(old_shortcut.clone(), *old_action);
                }
                *lock(&self.key_map) = restored;
                return Err(error.into());
            }
            registered.insert(shortcut.clone(), *action);
        }
        *lock(&self.key_map) = registered;

        log::info!("Registered {} global shortcuts", registration.len());

        Ok(())
    }

    /// Check if the manager is currently enabled and listening.
    pub fn is_enabled(&self) -> bool {
        !lock(&self.key_map).is_empty()
    }

    /// Look up which action corresponds to a shortcut string.
    pub fn lookup_action(&self, shortcut: &str) -> Option<HotkeyAction> {
        lock(&self.key_map)
            .get(&normalize_shortcut(shortcut))
            .copied()
    }

    /// Temporarily release one OS shortcut before replaying the same key into
    /// the game. Without this pause, the synthetic KeyViewer toggle would be
    /// intercepted by this manager again instead of reaching 3DMigoto.
    pub fn suspend_shortcut(&self, app: &tauri::AppHandle, shortcut: &str) -> Result<(), AppError> {
        app.global_shortcut()
            .unregister(shortcut)
            .map_err(AppError::from)
    }

    /// Try to acquire the action lock (debounce + switch_lock).
    pub fn try_acquire(&self) -> bool {
        lock(&self.state).try_acquire()
    }

    /// Release the action lock after an action completes.
    pub fn release(&self) {
        lock(&self.state).release();
    }

    fn arm_shortcut_release(&self, shortcut: &str) -> Arc<tokio::sync::Notify> {
        let waiter = Arc::new(tokio::sync::Notify::new());
        lock(&self.release_waiters).insert(normalize_shortcut(shortcut), Arc::clone(&waiter));
        waiter
    }

    /// Called by the global-shortcut handler for a release event.
    pub fn on_shortcut_released(&self, shortcut: &str) {
        if let Some(waiter) = lock(&self.release_waiters).remove(&normalize_shortcut(shortcut)) {
            // `notify_one` keeps one permit if the task has not started
            // awaiting yet, avoiding a race with a fast key release.
            waiter.notify_one();
        }
    }

    async fn wait_for_shortcut_release(waiter: Arc<tokio::sync::Notify>) -> Result<(), AppError> {
        tokio::time::timeout(Duration::from_millis(1_000), waiter.notified())
            .await
            .map_err(|_| {
                AppError::Validation(
                    "NeedsManualReload: triggering shortcut was not released".to_string(),
                )
            })
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
            let release_waiter = self.arm_shortcut_release(shortcut);
            tauri::async_runtime::spawn(async move {
                let result = match Self::wait_for_shortcut_release(release_waiter).await {
                    Ok(()) => execute_cycle_preset(&app_handle, direction).await,
                    Err(error) => Err(error),
                };
                match result {
                    Ok(summary) => log::info!("Hotkey {:?} → {}", action, summary),
                    Err(error) => {
                        crate::modules::system::application::telemetry::record_background_failure(
                            &app_handle,
                            &error,
                        )
                        .await;
                        log::error!("Preset cycle hotkey {:?} failed: {error}", action);
                    }
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });

            return;
        }

        if action == HotkeyAction::ToggleSafeMode {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            let release_waiter = self.arm_shortcut_release(shortcut);
            tauri::async_runtime::spawn(async move {
                let result = match Self::wait_for_shortcut_release(release_waiter).await {
                    Ok(()) => super::safe_mode::execute_toggle_safe_mode(&app_handle).await,
                    Err(error) => Err(error),
                };
                match result {
                    Ok(summary) => log::info!("Hotkey {:?} → {}", action, summary),
                    Err(error) => {
                        crate::modules::system::application::telemetry::record_background_failure(
                            &app_handle,
                            &error,
                        )
                        .await;
                        log::error!("Safe Mode hotkey failed: {error}");
                    }
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });
            return;
        }

        if action == HotkeyAction::ToggleOverlay {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            let release_waiter = self.arm_shortcut_release(shortcut);
            let shortcut_name = shortcut.to_string();
            let binding =
                get_key_string(&settings.hotkeys, HotkeyAction::ToggleOverlay).to_string();
            tauri::async_runtime::spawn(async move {
                let result = match Self::wait_for_shortcut_release(release_waiter).await {
                    Ok(()) => {
                        let suspend_result = match app_handle.try_state::<HotkeyManager>() {
                            Some(hotkey_manager) => hotkey_manager
                                .inner()
                                .suspend_shortcut(&app_handle, &shortcut_name),
                            None => Err(AppError::Internal(
                                "HotkeyManager is unavailable".to_string(),
                            )),
                        };
                        match suspend_result {
                            Ok(()) => {
                                let result = match app_handle.try_state::<ConfigService>() {
                                    Some(config_state) => {
                                        // Re-read settings and focus after release. The user may have
                                        // alt-tabbed while the physical shortcut was held.
                                        super::reload::trigger_binding_while_focused(
                                            &config_state.get_settings(),
                                            &binding,
                                        )
                                    }
                                    None => Err(AppError::Internal(
                                        "ConfigService is unavailable".to_string(),
                                    )),
                                };
                                if let Some(config_state) = app_handle.try_state::<ConfigService>()
                                {
                                    if let Some(hotkey_manager) =
                                        app_handle.try_state::<HotkeyManager>()
                                    {
                                        if let Err(error) = hotkey_manager.inner().update_bindings(
                                            &app_handle,
                                            &config_state.get_settings().hotkeys,
                                        ) {
                                            log::error!(
                                                "Could not restore global hotkeys after overlay replay: {error}"
                                            );
                                        }
                                    }
                                }
                                result
                            }
                            Err(error) => Err(error),
                        }
                    }
                    Err(error) => Err(error),
                };
                match result {
                    Ok(()) => log::info!("Hotkey {:?} replayed to the active game", action),
                    Err(error) => {
                        crate::modules::system::application::telemetry::record_background_failure(
                            &app_handle,
                            &error,
                        )
                        .await;
                        log::error!("Overlay hotkey failed: {error}");
                    }
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });
        }
    }
}

/// Validate every binding exposed in Settings and registered with the OS.
pub(crate) fn validate_binding_configuration(config: &HotkeyConfig) -> Result<(), AppError> {
    let mut configured = HashSet::new();
    for action in HotkeyAction::ALL {
        let shortcut = super::validate_3dmigoto_binding(get_key_string(config, action))?;
        if !configured.insert(shortcut) {
            return Err(AppError::Validation(
                "Hotkey bindings must not use the same shortcut".to_string(),
            ));
        }
    }
    for action in HotkeyAction::OS_ACTIONS {
        parse_hotkey(get_key_string(config, action))?;
    }
    Ok(())
}
