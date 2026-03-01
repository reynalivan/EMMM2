//! Integration tests for HotkeyManager — key parsing, registration maps, and dispatch logic.
//!
//! NOTE: These tests do NOT test OS-level hotkey registration (requires a desktop
//! session). They test the parsing, wiring, and dispatch logic only.

use crate::services::hotkeys::manager::{parse_hotkey, HotkeyManager};
use crate::services::hotkeys::{HotkeyAction, HotkeyConfig};

// ─── Key String Parsing ──────────────────────────────────────────────────────

#[test]
fn parse_single_key() {
    let hk = parse_hotkey("F5").expect("F5 should parse");
    assert_ne!(hk.id(), 0);
}

#[test]
fn parse_modifier_key() {
    let hk = parse_hotkey("Shift+F6").expect("Shift+F6 should parse");
    assert_ne!(hk.id(), 0);
}

#[test]
fn parse_case_insensitive() {
    let a = parse_hotkey("shift+F6").expect("lowercase shift");
    let b = parse_hotkey("SHIFT+F6").expect("uppercase SHIFT");
    // Both parse to the same hotkey ID
    assert_eq!(a.id(), b.id());
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
    // We can't create HotkeyManager in tests (requires OS manager),
    // so test the lookup/dispatch logic through the unit-level functions.
    // This test validates that unknown IDs produce None.
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return, // Skip on CI/headless
    };

    let result = manager.dispatch(
        999_999, // Fake ID that no hotkey maps to
        false,
        None,
        &[],
        0,
        0,
    );
    assert!(result.is_none());
}

#[test]
fn dispatch_when_disabled_returns_none() {
    let config = HotkeyConfig {
        enabled: false,
        ..Default::default()
    };
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return,
    };

    // Any ID should return None when disabled
    let result = manager.dispatch(1, false, None, &[], 0, 0);
    assert!(result.is_none());
}

#[test]
fn lookup_action_returns_correct_mapping() {
    let config = HotkeyConfig::default();
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return,
    };

    // Parse the same key to get the expected ID
    let f5_hk = parse_hotkey(&config.toggle_safe_mode).unwrap();
    let action = manager.lookup_action(f5_hk.id());
    assert_eq!(action, Some(HotkeyAction::ToggleSafeMode));

    let f6_hk = parse_hotkey(&config.next_preset).unwrap();
    let action = manager.lookup_action(f6_hk.id());
    assert_eq!(action, Some(HotkeyAction::NextPreset));
}

#[test]
fn update_bindings_clears_old_and_registers_new() {
    let initial_config = HotkeyConfig::default();
    let manager = match HotkeyManager::new(&initial_config) {
        Ok(m) => m,
        Err(_) => return,
    };

    // Initially enabled
    assert!(manager.is_enabled());

    // Update to disabled config
    let disabled = HotkeyConfig {
        enabled: false,
        ..Default::default()
    };
    manager.update_bindings(&disabled).unwrap();
    assert!(!manager.is_enabled());

    // Update back to enabled with different keys
    let new_config = HotkeyConfig {
        enabled: true,
        toggle_safe_mode: "F9".to_string(),
        ..Default::default()
    };
    manager.update_bindings(&new_config).unwrap();
    assert!(manager.is_enabled());

    // Old F5 should not map anymore, new F9 should
    let f5_hk = parse_hotkey("F5").unwrap();
    let f9_hk = parse_hotkey("F9").unwrap();
    assert_eq!(manager.lookup_action(f5_hk.id()), None);
    assert_eq!(
        manager.lookup_action(f9_hk.id()),
        Some(HotkeyAction::ToggleSafeMode)
    );
}

#[test]
fn dispatch_safe_mode_toggle() {
    let config = HotkeyConfig::default();
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return,
    };

    let f5_id = parse_hotkey(&config.toggle_safe_mode).unwrap().id();
    let result = manager.dispatch(f5_id, false, None, &[], 0, 0);

    assert!(result.is_some());
    let res = result.unwrap();
    assert!(res.summary.contains("Safe Mode"));
}

#[test]
fn dispatch_preset_cycle_with_presets() {
    let config = HotkeyConfig::default();
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return,
    };

    let f6_id = parse_hotkey(&config.next_preset).unwrap().id();
    let presets = vec!["Alpha".to_string(), "Beta".to_string(), "Gamma".to_string()];

    let result = manager.dispatch(f6_id, false, Some("Alpha"), &presets, 0, 0);

    assert!(result.is_some());
}

#[test]
fn dispatch_preset_cycle_no_presets_returns_noop() {
    let config = HotkeyConfig::default();
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return,
    };

    let f6_id = parse_hotkey(&config.next_preset).unwrap().id();
    let result = manager.dispatch(f6_id, false, None, &[], 0, 0);

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
    let manager = match HotkeyManager::new(&config) {
        Ok(m) => m,
        Err(_) => return,
    };

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
