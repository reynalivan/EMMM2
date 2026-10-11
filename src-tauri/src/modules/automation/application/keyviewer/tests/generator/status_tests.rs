use super::*;

#[test]
fn status_text_uses_the_runtime_safe_state_and_configured_bindings() {
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Maid Pack".to_string()),
        keyviewer_enabled: true,
        reload_binding: Some("F10".to_string()),
    };
    let text = generate_status_text(
        &fields,
        &crate::modules::automation::application::hotkeys::HotkeyConfig::default(),
    );

    assert_eq!(
        text,
        "Safe Mode: ON | Toggle: F5\nCollection: Maid Pack\nPrevious: SHIFT+F5 | Next: CTRL+F5\nKeyViewer: ON | Toggle: F7\nReload config: F10"
    );
}

#[test]
fn status_text_is_still_informative_without_an_active_preset() {
    let text = generate_status_text(
        &StatusFields::default(),
        &crate::modules::automation::application::hotkeys::HotkeyConfig::default(),
    );

    assert_eq!(
        text,
        "Safe Mode: OFF | Toggle: F5\nCollection: None\nPrevious: SHIFT+F5 | Next: CTRL+F5\nKeyViewer: inactive | Toggle: inactive (F7)\nReload config: manual reload required"
    );
    assert!(!text.contains("Runtime ready"));
}

#[test]
fn write_status_file_atomic() {
    let dir = TempDir::new().unwrap();
    let path = write_status_file(
        dir.path(),
        &StatusFields {
            preset_name: Some("Test".to_string()),
            ..Default::default()
        },
        &crate::modules::automation::application::hotkeys::HotkeyConfig::default(),
    )
    .unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Collection: Test"));
}

#[test]
fn disabled_controls_are_explicit_and_status_is_bounded() {
    let config = crate::modules::automation::application::hotkeys::HotkeyConfig {
        enabled: false,
        ..Default::default()
    };
    let text = generate_status_text(
        &StatusFields {
            preset_name: Some(format!("{}\nignored", "x".repeat(500))),
            keyviewer_enabled: true,
            reload_binding: None,
            ..Default::default()
        },
        &config,
    );

    assert!(text.contains("Toggle: inactive (F5)"));
    assert!(text.contains("Previous: inactive (SHIFT+F5)"));
    assert!(text.contains("KeyViewer: ON | Toggle: F7"));
    assert!(text.contains("manual reload required"));
    assert!(text.lines().count() <= 10);
    assert!(text.len() <= 4096);
}
