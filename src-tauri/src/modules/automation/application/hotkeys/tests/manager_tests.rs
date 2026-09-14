//! Unit tests for parsing and registration boundaries. Plugin registration is
//! intentionally covered by the Tauri integration layer, not this module.

use crate::modules::automation::application::hotkeys::manager::{
    parse_hotkey, validate_binding_configuration, HotkeyManager,
};
use crate::modules::automation::application::hotkeys::HotkeyConfig;

#[test]
fn parses_single_key() {
    assert_eq!(parse_hotkey("F5").unwrap(), "f5");
}

#[test]
fn parses_modifier_key_case_insensitively() {
    assert_eq!(parse_hotkey("Shift+F6").unwrap(), "shift+f6");
    assert_eq!(parse_hotkey("SHIFT+f6").unwrap(), "shift+f6");
}

#[test]
fn rejects_invalid_or_injectable_os_hotkeys() {
    for invalid in ["", "F7\n[Constants]", "F7;run = Evil", "F7=1", "Ctrl+F6++"] {
        assert!(parse_hotkey(invalid).is_err(), "must reject '{invalid}'");
    }
}

#[test]
fn rejects_3dmigoto_negative_modifiers_for_os_registration() {
    assert!(parse_hotkey("NO_CTRL+F5").is_err());
}

#[test]
fn permits_a_negative_modifier_on_the_3dmigoto_only_overlay_binding() {
    let config = HotkeyConfig {
        toggle_overlay: "NO_CTRL+F7".to_string(),
        ..Default::default()
    };
    assert!(validate_binding_configuration(&config).is_ok());
}

#[test]
fn manager_has_no_actions_until_tauri_registers_shortcuts() {
    let manager = HotkeyManager::new(&HotkeyConfig::default());
    assert_eq!(manager.lookup_action("f5"), None);
    assert_eq!(manager.lookup_action("shift+f6"), None);
}
