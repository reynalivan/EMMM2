//! Unit tests for hotkey configuration, debounce/switch_lock, actions, and conflict detection.

use std::thread;
use std::time::Duration;

use crate::services::hotkeys::actions::{
    plan_cycle_preset, plan_noop, resolve_next_preset, CycleDirection,
};
use crate::services::hotkeys::{
    detect_conflicts, get_key_string, list_bindings, HotkeyAction, HotkeyConfig, HotkeyState,
    KeyViewerConfig,
};

// ─── Config Defaults ─────────────────────────────────────────────────────────

#[test]
fn hotkey_config_defaults() {
    let config = HotkeyConfig::default();
    assert!(config.enabled);
    assert_eq!(config.cooldown_ms, 500);
    assert_eq!(config.next_preset, "Ctrl+F6");
    assert_eq!(config.prev_preset, "Shift+F6");
    assert_eq!(config.toggle_overlay, "F7");
}

#[test]
fn keyviewer_config_defaults() {
    let config = KeyViewerConfig::default();
    assert!(config.enabled);
}

#[test]
fn get_key_string_maps_correctly() {
    let config = HotkeyConfig::default();
    assert_eq!(get_key_string(&config, HotkeyAction::NextPreset), "Ctrl+F6");
    assert_eq!(
        get_key_string(&config, HotkeyAction::PrevPreset),
        "Shift+F6"
    );
    assert_eq!(get_key_string(&config, HotkeyAction::ToggleOverlay), "F7");
}

#[test]
fn list_bindings_returns_correct_count() {
    let config = HotkeyConfig::default();
    let bindings = list_bindings(&config);
    assert_eq!(bindings.len(), 5);
    assert_eq!(
        get_key_string(&config, HotkeyAction::NextVariantFolder),
        "Ctrl+F8"
    );
    assert_eq!(
        get_key_string(&config, HotkeyAction::PrevVariantFolder),
        "Shift+F8"
    );
}

// ... existing conflict and debounce tests ...

// ─── Conflict Detection ──────────────────────────────────────────────────────

#[test]
fn no_conflicts_with_default_config() {
    let config = HotkeyConfig::default();
    let conflicts = detect_conflicts(&config);
    assert!(conflicts.is_empty());
}

#[test]
fn detects_conflict_when_same_key() {
    let config = HotkeyConfig {
        prev_preset: "Ctrl+F6".to_string(), // Same as next_preset
        ..Default::default()
    };
    let conflicts = detect_conflicts(&config);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].2, "Ctrl+F6");
}

#[test]
fn conflict_detection_case_insensitive() {
    let config = HotkeyConfig {
        next_preset: "ctrl+f6".to_string(),
        prev_preset: "CTRL+F6".to_string(),
        ..Default::default()
    };
    let conflicts = detect_conflicts(&config);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].0, HotkeyAction::NextPreset);
    assert_eq!(conflicts[0].1, HotkeyAction::PrevPreset);
}

// ─── Debounce + Switch Lock ──────────────────────────────────────────────────

#[test]
fn first_acquire_succeeds() {
    let mut state = HotkeyState::new(500);
    assert!(state.try_acquire());
    assert!(state.is_locked());
}

#[test]
fn second_acquire_fails_while_locked() {
    let mut state = HotkeyState::new(500);
    assert!(state.try_acquire());
    assert!(!state.try_acquire()); // Dropped
}

#[test]
fn acquire_succeeds_after_release() {
    let mut state = HotkeyState::new(0); // no cooldown
    assert!(state.try_acquire());
    state.release();
    assert!(!state.is_locked());
    assert!(state.try_acquire());
}

#[test]
fn cooldown_prevents_rapid_retrigger() {
    let mut state = HotkeyState::new(200);
    assert!(state.try_acquire());
    state.release();
    // Immediately after release, cooldown should still be active
    assert!(state.is_cooldown_active());
    assert!(!state.try_acquire()); // Cooldown blocks
}

#[test]
fn cooldown_expires_allows_retrigger() {
    let mut state = HotkeyState::new(50); // 50ms cooldown
    assert!(state.try_acquire());
    state.release();
    thread::sleep(Duration::from_millis(60)); // Wait past cooldown
    assert!(state.try_acquire()); // Should work now
}

// ─── Safe Mode Toggle Action ─────────────────────────────────────────────────

// ─── Preset Cycling ──────────────────────────────────────────────────────────

#[test]
fn next_preset_wraps_around() {
    let presets = vec!["Alpha".into(), "Beta".into(), "Gamma".into()];
    let next = resolve_next_preset(&presets, Some("Gamma"), CycleDirection::Next);
    assert_eq!(next, Some("Alpha".to_string()));
}

#[test]
fn prev_preset_wraps_around() {
    let presets = vec!["Alpha".into(), "Beta".into(), "Gamma".into()];
    let prev = resolve_next_preset(&presets, Some("Alpha"), CycleDirection::Previous);
    assert_eq!(prev, Some("Gamma".to_string()));
}

#[test]
fn next_preset_alphabetical_order() {
    let presets = vec!["Zebra".into(), "Apple".into(), "Mango".into()];
    // Sorted: Apple, Mango, Zebra. Currently on "Apple" → Next → "Mango"
    let next = resolve_next_preset(&presets, Some("Apple"), CycleDirection::Next);
    assert_eq!(next, Some("Mango".to_string()));
}

#[test]
fn no_presets_returns_none() {
    let presets: Vec<String> = vec![];
    let next = resolve_next_preset(&presets, None, CycleDirection::Next);
    assert!(next.is_none());
}

#[test]
fn unknown_current_starts_at_first() {
    let presets = vec!["Beta".into(), "Alpha".into()];
    // Sorted: Alpha, Beta. Unknown current → start at 0 (Alpha)
    let next = resolve_next_preset(&presets, Some("Unknown"), CycleDirection::Next);
    assert_eq!(next, Some("Alpha".to_string()));
}

#[test]
fn unicode_preset_cycle_matches_current_name_with_ascii_case_fold_only() {
    let presets = vec![
        "프리셋_日本語".into(),
        "Preset_日本語".into(),
        "中文Preset".into(),
    ];
    let next = resolve_next_preset(&presets, Some("preset_日本語"), CycleDirection::Next);

    assert_eq!(next, Some("中文Preset".to_string()));
}

#[test]
fn plan_cycle_preset_sets_status() {
    let result = plan_cycle_preset("MyPreset", true);
    assert_eq!(result.status.preset_name, Some("MyPreset".to_string()));
    assert!(result.status.safe_mode);
    assert!(result.needs_reload);
}

// ─── Variant Folder Cycling ──────────────────────────────────────────────────

// ─── No-op ───────────────────────────────────────────────────────────────────

#[test]
fn noop_has_no_reload() {
    let result = plan_noop(HotkeyAction::NextPreset, "No presets configured", true);
    assert!(!result.needs_reload);
    assert!(result.summary.contains("No presets"));
}
