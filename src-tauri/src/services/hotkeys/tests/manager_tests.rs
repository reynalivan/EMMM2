//! Integration tests for HotkeyManager — key parsing, registration maps, and dispatch logic.
//! NOTE: These tests do NOT test plugin registration (requires app runtime).

use crate::services::hotkeys::manager::{parse_hotkey, HotkeyManager};
use crate::services::hotkeys::{HotkeyAction, HotkeyConfig};

// ─── Key String Parsing ──────────────────────────────────────────────────────

#[test]
fn parse_single_key() {
    let hk = parse_hotkey("F5").expect("F5 should parse");
    assert_eq!(hk, "f5");
}

#[test]
fn parse_modifier_key() {
    let hk = parse_hotkey("Shift+F6").expect("Shift+F6 should parse");
    assert_eq!(hk, "shift+f6");
}

#[test]
fn parse_case_insensitive() {
    let a = parse_hotkey("shift+F6").expect("lowercase shift");
    let b = parse_hotkey("SHIFT+F6").expect("uppercase SHIFT");
    assert_eq!(a, b);
}

#[test]
fn parse_invalid_returns_error() {
    let result = parse_hotkey("");
    assert!(result.is_err());
}

#[test]
fn parse_every_default_binding() {
    let config = HotkeyConfig::default();
    let keys = [
        &config.toggle_safe_mode,
        &config.next_preset,
        &config.prev_preset,
        &config.next_variant,
        &config.prev_variant,
        &config.toggle_overlay,
    ];
    for key in keys {
        parse_hotkey(key).unwrap_or_else(|e| panic!("Failed to parse default key '{key}': {e}"));
    }
}

// ─── Dispatch Logic ──────────────────────────────────────────────────────────

#[test]
fn dispatch_unknown_id_returns_none() {
    let config = HotkeyConfig::default();
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    let result = manager.lookup_action("f5");
    assert!(result.is_none());
}

#[test]
fn dispatch_when_disabled_returns_none() {
    let config = HotkeyConfig {
        enabled: false,
        ..Default::default()
    };
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    // Any action should return None when disabled.
    let result = manager.dispatch_action(HotkeyAction::ToggleSafeMode, false, None, &[]);
    assert!(result.is_none());
}

#[test]
fn lookup_action_returns_none_without_runtime_registration() {
    let config = HotkeyConfig::default();
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    // Shortcuts are populated only after runtime plugin registration.
    assert_eq!(manager.lookup_action("f5"), None);
    assert_eq!(manager.lookup_action("shift+f6"), None);
}

#[test]
fn dispatch_safe_mode_toggle() {
    let config = HotkeyConfig::default();
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    // Simulate already-enabled runtime manager.
    manager.set_enabled_for_test(true);

    let result = manager.dispatch_action(HotkeyAction::ToggleSafeMode, false, None, &[]);

    assert!(result.is_some());
    let res = result.unwrap();
    assert!(res.summary.contains("Safe Mode"));
}

#[test]
fn dispatch_preset_cycle_with_presets() {
    let config = HotkeyConfig::default();
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    manager.set_enabled_for_test(true);

    let presets = vec!["Alpha".to_string(), "Beta".to_string(), "Gamma".to_string()];

    let result = manager.dispatch_action(HotkeyAction::NextPreset, false, Some("Alpha"), &presets);

    assert!(result.is_some());
}

#[test]
fn dispatch_preset_cycle_no_presets_returns_noop() {
    let config = HotkeyConfig::default();
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    manager.set_enabled_for_test(true);

    let result = manager.dispatch_action(HotkeyAction::NextPreset, false, None, &[]);

    assert!(result.is_some());
    let res = result.unwrap();
    assert!(res.summary.contains("noop") || res.summary.contains("No presets"));
}

#[test]
fn try_acquire_and_release_cycle() {
    let config = HotkeyConfig {
        cooldown_ms: 0, // No cooldown for testing
        ..Default::default()
    };
    let manager = HotkeyManager::new(&config).expect("manager should initialize");

    // First acquire should succeed
    assert!(manager.try_acquire());
    // Second should fail (locked)
    assert!(!manager.try_acquire());
    // Release
    manager.release();
    // Third should succeed again
    assert!(manager.try_acquire());
    manager.release();
}
