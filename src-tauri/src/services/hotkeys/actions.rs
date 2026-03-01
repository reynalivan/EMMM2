//! Action handlers for hotkey-triggered operations.
//!
//! Each action follows the req-42 pipeline:
//! 1. Validate preconditions
//! 2. Compute new state
//! 3. Regenerate artifacts
//! 4. Apply workspace changes atomically
//! 5. Trigger 3DMigoto reload
//! 6. Write status banner
//! 7. Schedule banner clear after TTL
//!
//! The actual workspace/reload/privacy calls are abstracted behind
//! result types so callers can wire them to real services.

use super::HotkeyAction;
use crate::services::keyviewer::generator::StatusFields;

// ─── Action Result ───────────────────────────────────────────────────────────

/// The outcome of a hotkey action. Contains the new state and what needs to happen.
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Which action was executed.
    pub action: HotkeyAction,
    /// Status fields to write to runtime_status.txt.
    pub status: StatusFields,
    /// Whether a 3DMigoto reload should be triggered.
    pub needs_reload: bool,
    /// Human-readable summary for logging/toast.
    pub summary: String,
}

// ─── Safe Mode Toggle ────────────────────────────────────────────────────────

/// Compute the result of toggling Safe Mode.
///
/// The caller is responsible for actually flipping the mode via `PrivacyManager`,
/// recomputing the mod set, and applying the workspace.
pub fn plan_toggle_safe_mode(
    current_safe_mode: bool,
    current_preset_name: Option<&str>,
) -> ActionResult {
    let new_safe_mode = !current_safe_mode;
    let safe_label = if new_safe_mode { "ON" } else { "OFF" };

    ActionResult {
        action: HotkeyAction::ToggleSafeMode,
        status: StatusFields {
            safe_mode: new_safe_mode,
            preset_name: current_preset_name.map(|s| s.to_string()),
            ..Default::default()
        },
        needs_reload: true,
        summary: format!("Safe Mode: {safe_label}"),
    }
}

// ─── Preset Cycling ──────────────────────────────────────────────────────────

/// Direction for cycling through presets or folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleDirection {
    Next,
    Previous,
}

/// Compute which preset to switch to when cycling.
///
/// Returns `None` if there are no presets to cycle through.
/// Uses alphabetical ordering with wrap-around.
pub fn resolve_next_preset(
    preset_names: &[String],
    current_preset_name: Option<&str>,
    direction: CycleDirection,
) -> Option<String> {
    if preset_names.is_empty() {
        return None;
    }

    let mut sorted = preset_names.to_vec();
    sorted.sort();

    let current_idx = current_preset_name.and_then(|name| sorted.iter().position(|p| p == name));

    let next_idx = match (current_idx, direction) {
        (Some(idx), CycleDirection::Next) => (idx + 1) % sorted.len(),
        (Some(idx), CycleDirection::Previous) => {
            if idx == 0 {
                sorted.len() - 1
            } else {
                idx - 1
            }
        }
        (None, _) => 0, // No current → start at first
    };

    Some(sorted[next_idx].clone())
}

/// Compute the result of switching presets.
pub fn plan_cycle_preset(new_preset_name: &str, safe_mode: bool) -> ActionResult {
    ActionResult {
        action: HotkeyAction::NextPreset,
        status: StatusFields {
            safe_mode,
            preset_name: Some(new_preset_name.to_string()),
            ..Default::default()
        },
        needs_reload: true,
        summary: format!("Preset: {new_preset_name}"),
    }
}

// ─── Variant Folder Cycling ──────────────────────────────────────────────────

/// Compute which folder index to switch to when cycling.
///
/// Returns `None` if there are no folders to cycle through.
/// Uses wrap-around.
pub fn resolve_next_folder_index(
    folder_count: usize,
    current_index: usize,
    direction: CycleDirection,
) -> Option<usize> {
    if folder_count == 0 {
        return None;
    }

    let next = match direction {
        CycleDirection::Next => (current_index + 1) % folder_count,
        CycleDirection::Previous => {
            if current_index == 0 {
                folder_count - 1
            } else {
                current_index - 1
            }
        }
    };

    Some(next)
}

/// Compute the result of switching variant folders.
pub fn plan_cycle_variant(
    folder_name: &str,
    scope_name: &str,
    safe_mode: bool,
    preset_name: Option<&str>,
) -> ActionResult {
    ActionResult {
        action: HotkeyAction::NextVariantFolder,
        status: StatusFields {
            safe_mode,
            preset_name: preset_name.map(|s| s.to_string()),
            folder_name: Some(folder_name.to_string()),
            scope_name: Some(scope_name.to_string()),
        },
        needs_reload: true,
        summary: format!("Folder: {folder_name} (Scope: {scope_name})"),
    }
}

// ─── No-op Results ───────────────────────────────────────────────────────────

/// Generate a no-op result when an action can't be performed (e.g. no presets).
pub fn plan_noop(action: HotkeyAction, reason: &str, safe_mode: bool) -> ActionResult {
    ActionResult {
        action,
        status: StatusFields {
            safe_mode,
            ..Default::default()
        },
        needs_reload: false,
        summary: reason.to_string(),
    }
}
