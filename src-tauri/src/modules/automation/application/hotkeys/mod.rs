//! OS-level global hotkey manager — types, configuration, and debounce/lock logic.
//!
//! This module owns the testable business logic for hotkey handling:
//! - `HotkeyAction` enum for dispatching
//! - `HotkeyConfig` for user-configurable key bindings
//! - `HotkeyState` for debounce/cooldown + `switch_lock` mutual exclusion
//!
//! OS integration is owned by `manager.rs` through Tauri's global-shortcut
//! plugin; reload chords are replayed through `reload.rs` only after the shared
//! system game detector confirms the game window is focused.

pub mod cycle_preset;
pub mod focus;
pub mod manager;
pub mod reload;
pub mod safe_mode;

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};

// ─── Action Types ────────────────────────────────────────────────────────────

/// All actions that can be triggered by a global hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum HotkeyAction {
    /// Apply or remove the per-game Safe Mode filter (default: F5).
    ToggleSafeMode,
    /// Switch to next Collection preset (default: Ctrl+F6).
    NextPreset,
    /// Switch to previous Collection preset (default: Shift+F6).
    PrevPreset,
    /// Toggle KeyViewer overlay visibility (default: F7).
    ToggleOverlay,
}

impl HotkeyAction {
    /// Every action exposed in Settings and registered while global hotkeys
    /// are enabled. Overlay input is replayed only after the active game has
    /// been confirmed as the foreground process.
    pub const ALL: [HotkeyAction; 4] = [
        HotkeyAction::ToggleSafeMode,
        HotkeyAction::NextPreset,
        HotkeyAction::PrevPreset,
        HotkeyAction::ToggleOverlay,
    ];

    /// The subset that Tauri registers as OS-level shortcuts.
    pub const OS_ACTIONS: [HotkeyAction; 4] = [
        HotkeyAction::ToggleSafeMode,
        HotkeyAction::NextPreset,
        HotkeyAction::PrevPreset,
        HotkeyAction::ToggleOverlay,
    ];
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Hotkey configuration — persisted in AppSettings.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct HotkeyConfig {
    /// Whether hotkeys are globally enabled.
    pub enabled: bool,
    /// Key binding strings (e.g. "F6", "Shift+F6").
    #[serde(default = "default_safe_mode_key")]
    pub safe_mode: String,
    #[serde(default = "default_next_preset_key")]
    pub next_preset: String,
    #[serde(default = "default_prev_preset_key")]
    pub prev_preset: String,
    #[serde(default = "default_overlay_key")]
    pub toggle_overlay: String,
}

fn default_safe_mode_key() -> String {
    "F5".to_string()
}

fn default_next_preset_key() -> String {
    "Ctrl+F6".to_string()
}

fn default_prev_preset_key() -> String {
    "Shift+F6".to_string()
}

fn default_overlay_key() -> String {
    "F7".to_string()
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            safe_mode: default_safe_mode_key(),
            next_preset: default_next_preset_key(),
            prev_preset: default_prev_preset_key(),
            toggle_overlay: default_overlay_key(),
        }
    }
}

const HOTKEY_MUTATION_COOLDOWN: Duration = Duration::from_millis(500);

/// KeyViewer-specific configuration — persisted in AppSettings.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct KeyViewerConfig {
    /// Whether KeyViewer generation is enabled.
    pub enabled: bool,
}

impl Default for KeyViewerConfig {
    fn default() -> Self {
        Self { enabled: true }
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
    /// Create the state with the fixed mutation cooldown.
    pub fn new() -> Self {
        Self {
            switch_lock: false,
            last_trigger: None,
            cooldown: HOTKEY_MUTATION_COOLDOWN,
        }
    }

    #[cfg(test)]
    fn new_for_test(cooldown: Duration) -> Self {
        Self {
            switch_lock: false,
            last_trigger: None,
            cooldown,
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
}

impl Default for HotkeyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Map an action enum to its key string from the config. The only place the
/// action → config-field mapping is written; everything else derives from it.
pub fn get_key_string(config: &HotkeyConfig, action: HotkeyAction) -> &str {
    match action {
        HotkeyAction::ToggleSafeMode => &config.safe_mode,
        HotkeyAction::NextPreset => &config.next_preset,
        HotkeyAction::PrevPreset => &config.prev_preset,
        HotkeyAction::ToggleOverlay => &config.toggle_overlay,
    }
}

/// Validate and normalize a binding before it is registered or emitted into a
/// generated 3DMigoto `key =` line. Settings are untrusted input: a newline or
/// INI delimiter here could add arbitrary directives to the overlay file.
pub fn validate_3dmigoto_binding(key_str: &str) -> Result<String, AppError> {
    if key_str.chars().any(|character| {
        character.is_control() || matches!(character, ';' | '#' | '=' | '\\' | '/')
    }) {
        return Err(AppError::Validation(
            "Hotkey contains a disallowed control character or INI delimiter".to_string(),
        ));
    }

    let normalized = key_str.trim().replace(' ', "").to_ascii_lowercase();
    let tokens: Vec<&str> = normalized.split('+').collect();
    if normalized.is_empty() || tokens.iter().any(|token| token.is_empty()) {
        return Err(AppError::Validation("Hotkey cannot be empty".to_string()));
    }
    if tokens.len() > 4 {
        return Err(AppError::Validation(
            "Hotkey has too many modifiers".to_string(),
        ));
    }

    let (main, modifiers) = tokens
        .split_last()
        .expect("validated non-empty shortcut has a main key");
    let mut seen_modifiers = std::collections::HashSet::new();
    for modifier in modifiers {
        if !matches!(
            *modifier,
            "ctrl"
                | "control"
                | "shift"
                | "alt"
                | "meta"
                | "win"
                | "super"
                | "no_ctrl"
                | "no_shift"
                | "no_alt"
        ) {
            return Err(AppError::Validation(format!(
                "Unsupported hotkey modifier '{modifier}'"
            )));
        }
        if !seen_modifiers.insert(*modifier) {
            return Err(AppError::Validation(format!(
                "Hotkey repeats modifier '{modifier}'"
            )));
        }
    }

    let valid_main = matches!(*main, "[" | "]")
        || main
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if !valid_main
        || matches!(
            *main,
            "ctrl"
                | "control"
                | "shift"
                | "alt"
                | "meta"
                | "win"
                | "super"
                | "no_ctrl"
                | "no_shift"
                | "no_alt"
        )
    {
        return Err(AppError::Validation(format!(
            "Unsupported hotkey main key '{main}'"
        )));
    }

    Ok(normalized)
}

/// List all configurable hotkey actions with their current bindings.
pub fn list_bindings(config: &HotkeyConfig) -> Vec<(HotkeyAction, String)> {
    HotkeyAction::ALL
        .into_iter()
        .map(|action| (action, get_key_string(config, action).to_string()))
        .collect()
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
