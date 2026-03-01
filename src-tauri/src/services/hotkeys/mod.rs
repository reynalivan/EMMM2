//! OS-level global hotkey manager — types, configuration, and debounce/lock logic.
//!
//! This module owns the testable business logic for hotkey handling:
//! - `HotkeyAction` enum for dispatching
//! - `HotkeyConfig` for user-configurable key bindings
//! - `HotkeyState` for debounce/cooldown + `switch_lock` mutual exclusion
//!
//! **OS Integration (Phase 4b):** The actual `global-hotkey` crate registration
//! and `enigo`/`windows-sys` keystroke sending will be added when their
//! Cargo.toml dependencies are introduced.

pub mod actions;
pub mod manager;

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// ─── Action Types ────────────────────────────────────────────────────────────

/// All actions that can be triggered by a global hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyAction {
    /// Toggle Safe Mode on/off (default: F5).
    ToggleSafeMode,
    /// Switch to next Collection preset (default: F6).
    NextPreset,
    /// Switch to previous Collection preset (default: Shift+F6).
    PrevPreset,
    /// Switch to next variant folder (default: F8).
    NextVariantFolder,
    /// Switch to previous variant folder (default: Shift+F8).
    PrevVariantFolder,
    /// Toggle KeyViewer overlay visibility (default: F7).
    ToggleOverlay,
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Hotkey configuration — persisted in AppSettings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Whether hotkeys are globally enabled.
    pub enabled: bool,
    /// Only trigger hotkeys when the game window is focused.
    pub game_focus_only: bool,
    /// Cooldown between successive hotkey triggers (milliseconds).
    pub cooldown_ms: u64,
    /// Key binding strings (e.g. "F5", "Shift+F6").
    pub toggle_safe_mode: String,
    pub next_preset: String,
    pub prev_preset: String,
    pub next_variant: String,
    pub prev_variant: String,
    pub toggle_overlay: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            game_focus_only: true,
            cooldown_ms: 500,
            toggle_safe_mode: "F5".to_string(),
            next_preset: "F6".to_string(),
            prev_preset: "Shift+F6".to_string(),
            next_variant: "F8".to_string(),
            prev_variant: "Shift+F8".to_string(),
            toggle_overlay: "F7".to_string(),
        }
    }
}

/// KeyViewer-specific configuration — persisted in AppSettings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyViewerConfig {
    /// Whether KeyViewer generation is enabled.
    pub enabled: bool,
    /// Status banner TTL in seconds.
    pub status_ttl_seconds: f32,
    /// Toggle key for in-game overlay (written to KeyViewer.ini).
    pub overlay_toggle_key: String,
    /// Relative path from game mod root for keybind text files.
    pub keybinds_dir: String,
}

impl Default for KeyViewerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            status_ttl_seconds: 3.0,
            overlay_toggle_key: "F7".to_string(),
            keybinds_dir: "EMM2/keybinds/active".to_string(),
        }
    }
}

// ─── Debounce + Switch Lock ──────────────────────────────────────────────────

/// Manages debounce cooldown and mutual exclusion (switch_lock) for hotkey actions.
///
/// **Invariant:** While `switch_lock` is held, ALL incoming hotkey events are dropped.
/// The cooldown prevents rapid re-triggering even after lock release.
pub struct HotkeyState {
    /// Whether an action is currently executing (mutex gate).
    switch_lock: bool,
    /// Timestamp of the last accepted hotkey trigger.
    last_trigger: Option<Instant>,
    /// Cooldown duration.
    cooldown: Duration,
}

impl HotkeyState {
    /// Create a new `HotkeyState` with the given cooldown.
    pub fn new(cooldown_ms: u64) -> Self {
        Self {
            switch_lock: false,
            last_trigger: None,
            cooldown: Duration::from_millis(cooldown_ms),
        }
    }

    /// Try to acquire the switch lock for an action.
    ///
    /// Returns `true` if the action should proceed:
    /// - `switch_lock` is not held
    /// - Cooldown period has elapsed since last trigger
    ///
    /// Returns `false` (drop the input) if either guard fails.
    pub fn try_acquire(&mut self) -> bool {
        if self.switch_lock {
            return false;
        }

        if let Some(last) = self.last_trigger {
            if last.elapsed() < self.cooldown {
                return false;
            }
        }

        self.switch_lock = true;
        self.last_trigger = Some(Instant::now());
        true
    }

    /// Release the switch lock after action completes (success or failure).
    pub fn release(&mut self) {
        self.switch_lock = false;
    }

    /// Check if the switch lock is currently held.
    pub fn is_locked(&self) -> bool {
        self.switch_lock
    }

    /// Check if cooldown has elapsed since last trigger.
    pub fn is_cooldown_active(&self) -> bool {
        self.last_trigger
            .map(|t| t.elapsed() < self.cooldown)
            .unwrap_or(false)
    }

    /// Update the cooldown duration (e.g. after settings change).
    pub fn update_cooldown(&mut self, new_cooldown_ms: u64) {
        self.cooldown = Duration::from_millis(new_cooldown_ms);
    }
}

/// Map an action enum to its key string from the config.
pub fn get_key_string(config: &HotkeyConfig, action: HotkeyAction) -> &str {
    match action {
        HotkeyAction::ToggleSafeMode => &config.toggle_safe_mode,
        HotkeyAction::NextPreset => &config.next_preset,
        HotkeyAction::PrevPreset => &config.prev_preset,
        HotkeyAction::NextVariantFolder => &config.next_variant,
        HotkeyAction::PrevVariantFolder => &config.prev_variant,
        HotkeyAction::ToggleOverlay => &config.toggle_overlay,
    }
}

/// List all configurable hotkey actions with their current bindings.
pub fn list_bindings(config: &HotkeyConfig) -> Vec<(HotkeyAction, String)> {
    vec![
        (
            HotkeyAction::ToggleSafeMode,
            config.toggle_safe_mode.clone(),
        ),
        (HotkeyAction::NextPreset, config.next_preset.clone()),
        (HotkeyAction::PrevPreset, config.prev_preset.clone()),
        (HotkeyAction::NextVariantFolder, config.next_variant.clone()),
        (HotkeyAction::PrevVariantFolder, config.prev_variant.clone()),
        (HotkeyAction::ToggleOverlay, config.toggle_overlay.clone()),
    ]
}

/// Detect conflicts between hotkey bindings (same key used for multiple actions).
pub fn detect_conflicts(config: &HotkeyConfig) -> Vec<(HotkeyAction, HotkeyAction, String)> {
    let bindings = list_bindings(config);
    let mut conflicts = Vec::new();

    for i in 0..bindings.len() {
        for j in (i + 1)..bindings.len() {
            if bindings[i].1.eq_ignore_ascii_case(&bindings[j].1) {
                conflicts.push((bindings[i].0, bindings[j].0, bindings[i].1.clone()));
            }
        }
    }

    conflicts
}
