use super::*;

#[test]
fn status_text_uses_the_runtime_safe_state_and_configured_bindings() {
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Maid Pack".to_string()),
        ..Default::default()
    };
    let text = generate_status_text(
        &fields,
        &crate::modules::automation::application::hotkeys::HotkeyConfig::default(),
    );

    assert_eq!(
        text,
        "Safe: On [F5] | Preset: Maid Pack [SHIFT+F6] [CTRL+F6]"
    );
}

#[test]
fn status_text_is_still_informative_without_an_active_preset() {
    let text = generate_status_text(
        &StatusFields::default(),
        &crate::modules::automation::application::hotkeys::HotkeyConfig::default(),
    );

    assert_eq!(text, "Safe: Off [F5] | Preset: None [SHIFT+F6] [CTRL+F6]");
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
    assert!(content.contains("Preset: Test"));
}
