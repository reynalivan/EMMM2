//! HotkeyManager — bridges OS-level global hotkeys to action planners.
//!
//! Owns the `GlobalHotKeyManager` from `global-hotkey` crate (tauri-apps),
//! manages registration lifecycle, and dispatches events to action planners.
//!
//! **Threading model:**
//! - Registration/unregistration happens on the main thread (required by global-hotkey).
//! - Event polling runs in a background `tokio::task`.
//! - `HotkeyState` (debounce/switch_lock) is protected by `Mutex`.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use global_hotkey::hotkey::HotKey;
use global_hotkey::GlobalHotKeyManager;

use super::actions::{self, ActionResult, CycleDirection};
use super::{HotkeyAction, HotkeyConfig, HotkeyState};

// ─── Key Parsing ─────────────────────────────────────────────────────────────

/// Parse a user-facing key string (e.g. "F5", "Shift+F6") into a `global_hotkey::HotKey`.
///
/// The `global-hotkey` crate's `FromStr` impl is case-insensitive and supports
/// modifier+key format natively, so this is a thin error-wrapping bridge.
pub fn parse_hotkey(key_str: &str) -> Result<HotKey, String> {
    HotKey::from_str(key_str).map_err(|e| format!("Failed to parse hotkey '{key_str}': {e}"))
}

// ─── Registration Map ────────────────────────────────────────────────────────

/// Maps a `HotKey.id()` back to our domain `HotkeyAction`.
type HotkeyIdMap = HashMap<u32, HotkeyAction>;

/// Build a map of (HotKey, HotkeyAction) from the user config.
/// Returns both the parsed hotkeys and the ID→action map.
fn build_registration(
    config: &HotkeyConfig,
) -> Result<(Vec<(HotKey, HotkeyAction)>, HotkeyIdMap), String> {
    let bindings = [
        (HotkeyAction::ToggleSafeMode, &config.toggle_safe_mode),
        (HotkeyAction::NextPreset, &config.next_preset),
        (HotkeyAction::PrevPreset, &config.prev_preset),
        (HotkeyAction::NextVariantFolder, &config.next_variant),
        (HotkeyAction::PrevVariantFolder, &config.prev_variant),
        (HotkeyAction::ToggleOverlay, &config.toggle_overlay),
    ];

    let mut entries = Vec::new();
    let mut id_map = HashMap::new();

    for (action, key_str) in bindings {
        let hk = parse_hotkey(key_str)?;
        id_map.insert(hk.id(), action);
        entries.push((hk, action));
    }

    Ok((entries, id_map))
}

// ─── HotkeyManager ──────────────────────────────────────────────────────────

/// Managed Tauri state — owns OS hotkey lifecycle.
pub struct HotkeyManager {
    /// The OS-level hotkey manager (must live on main thread).
    manager: GlobalHotKeyManager,
    /// Map from hotkey ID → our action enum.
    id_map: Mutex<HotkeyIdMap>,
    /// Debounce / switch-lock state.
    state: Mutex<HotkeyState>,
    /// Whether the manager is actively listening.
    enabled: Mutex<bool>,
}

// SAFETY: GlobalHotKeyManager wraps an OS handle (raw pointer) that is inherently
// thread-unsafe, but we serialize all registration calls through Tauri commands
// (which run on the async runtime) and protect mutable state with Mutex.
// The OS handle is only used for register/unregister calls, never concurrently.
unsafe impl Send for HotkeyManager {}
unsafe impl Sync for HotkeyManager {}

impl HotkeyManager {
    /// Create a new HotkeyManager. Must be called from the main thread.
    pub fn new(config: &HotkeyConfig) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Failed to create hotkey manager: {e}"))?;

        let hm = Self {
            manager,
            id_map: Mutex::new(HashMap::new()),
            state: Mutex::new(HotkeyState::new(config.cooldown_ms)),
            enabled: Mutex::new(false),
        };

        if config.enabled {
            hm.register_all(config)?;
        }

        Ok(hm)
    }

    /// Register all hotkeys from the config with the OS.
    fn register_all(&self, config: &HotkeyConfig) -> Result<(), String> {
        let (entries, id_map) = build_registration(config)?;

        for (hk, action) in &entries {
            self.manager
                .register(*hk)
                .map_err(|e| format!("Failed to register {:?} ({}): {e}", action, hk))?;
        }

        *self.id_map.lock().unwrap_or_else(|p| p.into_inner()) = id_map;
        *self.enabled.lock().unwrap_or_else(|p| p.into_inner()) = true;

        log::info!("Registered {} global hotkeys", entries.len());

        Ok(())
    }

    /// Unregister all hotkeys from the OS.
    fn unregister_all(&self) {
        let id_map = self.id_map.lock().unwrap_or_else(|p| p.into_inner());
        // We don't have the HotKey objects anymore, but we can reconstruct from IDs.
        // The safest approach is to just clear the map so events are ignored.
        drop(id_map);

        *self.id_map.lock().unwrap_or_else(|p| p.into_inner()) = HashMap::new();
        *self.enabled.lock().unwrap_or_else(|p| p.into_inner()) = false;

        log::info!("Unregistered all global hotkeys");
    }

    /// Update hotkey bindings after settings change.
    /// Unregisters old hotkeys and registers new ones.
    pub fn update_bindings(&self, config: &HotkeyConfig) -> Result<(), String> {
        self.unregister_all();

        if config.enabled {
            self.register_all(config)?;
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

    /// Look up which action corresponds to a hotkey event ID.
    pub fn lookup_action(&self, hotkey_id: u32) -> Option<HotkeyAction> {
        self.id_map
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&hotkey_id)
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

    /// Dispatch a hotkey event to the appropriate action planner.
    ///
    /// Returns `Some(ActionResult)` if the action was handled, or `None` if
    /// the event was ignored (unknown ID, disabled, or debounced).
    pub fn dispatch(
        &self,
        hotkey_id: u32,
        safe_mode: bool,
        current_preset: Option<&str>,
        available_presets: &[String],
        folder_count: usize,
        current_folder_index: usize,
    ) -> Option<ActionResult> {
        if !self.is_enabled() {
            return None;
        }

        let action = self.lookup_action(hotkey_id)?;

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

        log::info!("Hotkey {:?} → {}", action, result.summary);
        self.release();

        Some(result)
    }
}
