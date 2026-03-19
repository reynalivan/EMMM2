//! HotkeyManager — bridges OS-level global hotkeys to action planners.
//!
//! Owns global shortcut registration lifecycle through `tauri-plugin-global-shortcut`,
//! and dispatches events to action planners.
//!
//! **Threading model:**
//! - Registration/unregistration happens through Tauri plugin APIs.
//! - Event listening is callback-driven via plugin handler (configured in `lib.rs`).
//! - `HotkeyState` (debounce/switch_lock) is protected by `Mutex`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::services::config::ConfigService;
use crate::services::keyviewer::generator::StatusFields;

use super::actions::{self, ActionResult, CycleDirection};
use super::focus;
use super::{HotkeyAction, HotkeyConfig, HotkeyState};

// ─── Key Parsing ─────────────────────────────────────────────────────────────

/// Parse and normalize a user-facing key string (e.g. "F5", "Shift+F6").
pub fn parse_hotkey(key_str: &str) -> Result<String, String> {
    let normalized = normalize_shortcut(key_str);
    if normalized.is_empty() {
        return Err("Hotkey cannot be empty".to_string());
    }

    let has_key_token = normalized
        .split('+')
        .map(str::trim)
        .any(|token| !token.is_empty());

    if !has_key_token {
        return Err(format!("Invalid hotkey '{key_str}'"));
    }

    Ok(normalized)
}

fn normalize_shortcut(key_str: &str) -> String {
    key_str.trim().replace(' ', "").to_ascii_lowercase()
}

// ─── Registration Map ────────────────────────────────────────────────────────

type HotkeyMap = HashMap<String, HotkeyAction>;

/// Build a map of (shortcut string, HotkeyAction) from the user config.
fn build_registration(config: &HotkeyConfig) -> Result<Vec<(String, HotkeyAction)>, String> {
    let bindings = [
        (HotkeyAction::ToggleSafeMode, &config.toggle_safe_mode),
        (HotkeyAction::NextPreset, &config.next_preset),
        (HotkeyAction::PrevPreset, &config.prev_preset),
        (HotkeyAction::NextVariantFolder, &config.next_variant),
        (HotkeyAction::PrevVariantFolder, &config.prev_variant),
        (HotkeyAction::ToggleOverlay, &config.toggle_overlay),
    ];

    let mut entries = Vec::new();

    for (action, key_str) in bindings {
        entries.push((parse_hotkey(key_str)?, action));
    }

    Ok(entries)
}

// ─── HotkeyManager ──────────────────────────────────────────────────────────

/// Managed Tauri state — owns OS hotkey lifecycle.
pub struct HotkeyManager {
    /// Map from normalized shortcut string → action enum.
    key_map: Mutex<HotkeyMap>,
    /// Debounce / switch-lock state.
    state: Mutex<HotkeyState>,
    /// Whether the manager is actively listening.
    enabled: Mutex<bool>,
}

impl HotkeyManager {
    /// Create a new HotkeyManager.
    pub fn new(config: &HotkeyConfig) -> Result<Self, String> {
        Ok(Self {
            key_map: Mutex::new(HashMap::new()),
            state: Mutex::new(HotkeyState::new(config.cooldown_ms)),
            enabled: Mutex::new(false),
        })
    }

    /// Register all shortcuts from the config with Tauri global shortcut plugin.
    fn register_all(&self, app: &tauri::AppHandle, config: &HotkeyConfig) -> Result<(), String> {
        let global_shortcut = app.global_shortcut();
        let entries = build_registration(config)?;
        let mut key_map = HashMap::new();

        global_shortcut
            .unregister_all()
            .map_err(|e| format!("Failed to clear existing shortcuts: {e}"))?;

        for (shortcut, action) in &entries {
            global_shortcut
                .register(shortcut.as_str())
                .map_err(|e| format!("Failed to register {:?} ({shortcut}): {e}", action))?;
            key_map.insert(shortcut.clone(), *action);
        }

        *self.key_map.lock().unwrap_or_else(|p| p.into_inner()) = key_map;
        *self.enabled.lock().unwrap_or_else(|p| p.into_inner()) = true;

        log::info!("Registered {} global shortcuts", entries.len());

        Ok(())
    }

    /// Unregister all shortcuts from the plugin.
    fn unregister_all(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let global_shortcut = app.global_shortcut();
        global_shortcut
            .unregister_all()
            .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;

        *self.key_map.lock().unwrap_or_else(|p| p.into_inner()) = HashMap::new();
        *self.enabled.lock().unwrap_or_else(|p| p.into_inner()) = false;

        log::info!("Unregistered all global shortcuts");

        Ok(())
    }

    /// Update shortcuts after settings change.
    /// Unregisters old shortcuts and registers new ones.
    pub fn update_bindings(
        &self,
        app: &tauri::AppHandle,
        config: &HotkeyConfig,
    ) -> Result<(), String> {
        self.unregister_all(app)?;

        if config.enabled {
            self.register_all(app, config)?;
        }

        // Update cooldown
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .update_cooldown(config.cooldown_ms);

        Ok(())
    }

    /// Check if the manager is currently enabled and listening.
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap_or_else(|p| p.into_inner())
    }

    #[cfg(test)]
    pub fn set_enabled_for_test(&self, enabled: bool) {
        *self.enabled.lock().unwrap_or_else(|p| p.into_inner()) = enabled;
    }

    /// Look up which action corresponds to a shortcut string.
    pub fn lookup_action(&self, shortcut: &str) -> Option<HotkeyAction> {
        self.key_map
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&normalize_shortcut(shortcut))
            .copied()
    }

    /// Try to acquire the action lock (debounce + switch_lock).
    pub fn try_acquire(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .try_acquire()
    }

    /// Release the action lock after an action completes.
    pub fn release(&self) {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .release();
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

        if settings.hotkeys.game_focus_only && !focus::is_active_game_focused(&settings) {
            return;
        }

        if action == HotkeyAction::ToggleSafeMode {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let current_safe_mode = settings.safe_mode.enabled;
            let summary = actions::plan_toggle_safe_mode(current_safe_mode, None).summary;
            log::info!("Hotkey {:?} → {}", action, summary);

            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = execute_toggle_safe_mode(&app_handle).await {
                    log::error!("Toggle safe mode hotkey failed: {e}");
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });

            return;
        }

        if matches!(action, HotkeyAction::NextPreset | HotkeyAction::PrevPreset) {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let result = match action {
                    HotkeyAction::NextPreset => {
                        execute_cycle_preset(&app_handle, CycleDirection::Next).await
                    }
                    HotkeyAction::PrevPreset => {
                        execute_cycle_preset(&app_handle, CycleDirection::Previous).await
                    }
                    _ => unreachable!("guarded by matches!"),
                };

                match result {
                    Ok(summary) => log::info!("Hotkey {:?} → {}", action, summary),
                    Err(e) => log::error!("Preset cycle hotkey {:?} failed: {e}", action),
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });

            return;
        }

        if matches!(
            action,
            HotkeyAction::NextVariantFolder | HotkeyAction::PrevVariantFolder
        ) {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let result = match action {
                    HotkeyAction::NextVariantFolder => {
                        execute_cycle_variant(&app_handle, CycleDirection::Next).await
                    }
                    HotkeyAction::PrevVariantFolder => {
                        execute_cycle_variant(&app_handle, CycleDirection::Previous).await
                    }
                    _ => unreachable!("guarded by matches!"),
                };

                match result {
                    Ok(summary) => log::info!("Hotkey {:?} → {}", action, summary),
                    Err(e) => log::error!("Variant cycle hotkey {:?} failed: {e}", action),
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });

            return;
        }

        let safe_mode = settings.safe_mode.enabled;
        if let Some(result) = self.dispatch_action(action, safe_mode, None, &[], 0, 0) {
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
        folder_count: usize,
        current_folder_index: usize,
    ) -> Option<ActionResult> {
        if !self.is_enabled() {
            return None;
        }

        if !self.try_acquire() {
            log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
            return None;
        }

        let result = match action {
            HotkeyAction::ToggleSafeMode => {
                actions::plan_toggle_safe_mode(safe_mode, current_preset)
            }
            HotkeyAction::NextPreset => {
                match actions::resolve_next_preset(
                    available_presets,
                    current_preset,
                    CycleDirection::Next,
                ) {
                    Some(next) => actions::plan_cycle_preset(&next, safe_mode),
                    None => actions::plan_noop(action, "No presets available", safe_mode),
                }
            }
            HotkeyAction::PrevPreset => {
                match actions::resolve_next_preset(
                    available_presets,
                    current_preset,
                    CycleDirection::Previous,
                ) {
                    Some(prev) => actions::plan_cycle_preset(&prev, safe_mode),
                    None => actions::plan_noop(action, "No presets available", safe_mode),
                }
            }
            HotkeyAction::NextVariantFolder => {
                match actions::resolve_next_folder_index(
                    folder_count,
                    current_folder_index,
                    CycleDirection::Next,
                ) {
                    Some(idx) => actions::plan_cycle_variant(
                        &format!("Folder {}", idx),
                        "Current",
                        safe_mode,
                        current_preset,
                    ),
                    None => actions::plan_noop(action, "No variant folders", safe_mode),
                }
            }
            HotkeyAction::PrevVariantFolder => {
                match actions::resolve_next_folder_index(
                    folder_count,
                    current_folder_index,
                    CycleDirection::Previous,
                ) {
                    Some(idx) => actions::plan_cycle_variant(
                        &format!("Folder {}", idx),
                        "Current",
                        safe_mode,
                        current_preset,
                    ),
                    None => actions::plan_noop(action, "No variant folders", safe_mode),
                }
            }
            HotkeyAction::ToggleOverlay => {
                // Overlay toggle is handled directly by 3DMigoto INI — no backend work needed.
                // Just emit the event status for logging purposes.
                actions::plan_noop(action, "Overlay toggle (handled by 3DMigoto)", safe_mode)
            }
        };

        self.release();

        Some(result)
    }
}

async fn execute_toggle_safe_mode(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(config_state) = app.try_state::<ConfigService>() else {
        return Err("ConfigService not available".to_string());
    };
    let Some(pool_state) = app.try_state::<sqlx::SqlitePool>() else {
        return Err("SqlitePool not available".to_string());
    };
    let Some(watcher_state) = app.try_state::<crate::services::scanner::watcher::WatcherState>()
    else {
        return Err("WatcherState not available".to_string());
    };
    let Some(op_lock) = app.try_state::<crate::services::fs_utils::operation_lock::OperationLock>()
    else {
        return Err("OperationLock not available".to_string());
    };

    let settings = config_state.get_settings();
    let game_id = settings
        .active_game_id
        .as_deref()
        .ok_or_else(|| "No active game selected".to_string())?;
    let target_enabled = !settings.safe_mode.enabled;
    let mode = if target_enabled {
        crate::services::privacy::Mode::SFW
    } else {
        crate::services::privacy::Mode::NSFW
    };

    let _lock = op_lock.inner().acquire().await?;

    crate::services::privacy::switch_mode(
        mode,
        pool_state.inner(),
        watcher_state.inner(),
        &game_id,
    )
    .await?;
    config_state.set_safe_mode_enabled(target_enabled)?;

    let settings_after = config_state.get_settings();
    let status = StatusFields {
        safe_mode: target_enabled,
        ..Default::default()
    };
    write_runtime_status(
        pool_state.inner(),
        &game_id,
        &status,
        settings_after.keyviewer.status_ttl_seconds,
    )
    .await?;

    let reload_key = super::reload::trigger_reload_fixes(&settings_after)?;
    log::debug!("Sent reload_fixes key after safe mode switch: {reload_key}");

    Ok(())
}

async fn execute_cycle_preset(
    app: &tauri::AppHandle,
    direction: CycleDirection,
) -> Result<String, String> {
    let Some(config_state) = app.try_state::<ConfigService>() else {
        return Err("ConfigService not available".to_string());
    };
    let Some(pool_state) = app.try_state::<sqlx::SqlitePool>() else {
        return Err("SqlitePool not available".to_string());
    };
    let Some(watcher_state) = app.try_state::<crate::services::scanner::watcher::WatcherState>()
    else {
        return Err("WatcherState not available".to_string());
    };
    let Some(op_lock) = app.try_state::<crate::services::fs_utils::operation_lock::OperationLock>()
    else {
        return Err("OperationLock not available".to_string());
    };

    let settings = config_state.get_settings();
    let game_id = settings
        .active_game_id
        .as_deref()
        .ok_or_else(|| "No active game selected".to_string())?;
    let safe_mode_enabled = settings.safe_mode.enabled;

    let collections = crate::services::collections::list_collections(
        pool_state.inner(),
        &game_id,
        safe_mode_enabled,
    )
    .await?;

    if collections.is_empty() {
        let status = StatusFields {
            safe_mode: safe_mode_enabled,
            preset_name: Some("No presets configured".to_string()),
            ..Default::default()
        };
        write_runtime_status(
            pool_state.inner(),
            &game_id,
            &status,
            settings.keyviewer.status_ttl_seconds,
        )
        .await?;
        return Ok("No presets available".to_string());
    }

    let preset_names: Vec<String> = collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect();
    let runtime_snapshot = crate::services::corridor_runtime::get_corridor_runtime_snapshot(
        pool_state.inner(),
        &game_id,
        safe_mode_enabled,
    )
    .await?;
    let current_name = if runtime_snapshot.state_kind
        == crate::services::collections::CollectionStateKind::Named
    {
        runtime_snapshot.state_name.as_deref()
    } else {
        None
    };
    let target_name = actions::resolve_next_preset(&preset_names, current_name, direction)
        .ok_or_else(|| "No presets available".to_string())?;

    let target = collections
        .iter()
        .find(|collection| collection.name == target_name)
        .ok_or_else(|| format!("Target preset '{target_name}' not found"))?;

    let _lock = op_lock.inner().acquire().await?;

    let apply_result = crate::services::collections::apply_collection(
        pool_state.inner(),
        watcher_state.inner(),
        &target.id,
        &game_id,
        safe_mode_enabled,
    )
    .await?;

    let planner = actions::plan_cycle_preset(&target.name, safe_mode_enabled);

    write_runtime_status(
        pool_state.inner(),
        &game_id,
        &planner.status,
        settings.keyviewer.status_ttl_seconds,
    )
    .await?;

    let reload_key = super::reload::trigger_reload_fixes(&settings)?;

    Ok(format!(
        "{} (changed: {}, reload: {})",
        planner.summary, apply_result.changed_count, reload_key
    ))
}

async fn execute_cycle_variant(
    app: &tauri::AppHandle,
    direction: CycleDirection,
) -> Result<String, String> {
    let Some(config_state) = app.try_state::<ConfigService>() else {
        return Err("ConfigService not available".to_string());
    };
    let Some(pool_state) = app.try_state::<sqlx::SqlitePool>() else {
        return Err("SqlitePool not available".to_string());
    };

    let settings = config_state.get_settings();
    let game_id = settings
        .active_game_id
        .ok_or_else(|| "No active game selected".to_string())?;

    let direction_label = match direction {
        CycleDirection::Next => "Next",
        CycleDirection::Previous => "Previous",
    };

    let status = StatusFields {
        safe_mode: settings.safe_mode.enabled,
        preset_name: None,
        folder_name: Some("No variant group".to_string()),
        scope_name: Some("Current scope".to_string()),
    };

    write_runtime_status(
        pool_state.inner(),
        &game_id,
        &status,
        settings.keyviewer.status_ttl_seconds,
    )
    .await?;

    Ok(format!(
        "{direction_label} variant ignored: no variant group for current scope"
    ))
}

async fn write_runtime_status(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    status: &StatusFields,
    ttl_seconds: f32,
) -> Result<(), String> {
    let Some(mods_path) = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?
    else {
        return Ok(());
    };

    let status_dir = Path::new(&mods_path).join("EMM2").join("status");
    crate::services::keyviewer::generator::write_status_file(&status_dir, status)?;

    if ttl_seconds > 0.0 {
        let clear_dir = status_dir.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs_f32(ttl_seconds)).await;
            if let Err(e) = crate::services::keyviewer::generator::clear_status_file(&clear_dir) {
                log::warn!("Failed to clear runtime_status.txt: {e}");
            }
        });
    }

    Ok(())
}
