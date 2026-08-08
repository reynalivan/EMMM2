use super::*;

// ─── Status Banner ───────────────────────────────────────────────────────────

#[test]
fn status_text_includes_preset_without_safe_mode() {
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Default".to_string()),
        conflict_count: Some(0),
        ..Default::default()
    };
    let text = generate_status_text(&fields, &crate::services::hotkeys::HotkeyConfig::default());
    assert!(text.contains("Preset: Default"));
    assert!(!text.contains("Safe:"));
}

#[test]
fn status_text_with_folder() {
    let fields = StatusFields {
        safe_mode: false,
        preset_name: Some("Main".to_string()),
        folder_name: Some("Cape".to_string()),
        scope_name: Some("Albedo".to_string()),
        conflict_count: Some(0),
    };
    let text = generate_status_text(&fields, &crate::services::hotkeys::HotkeyConfig::default());
    assert!(text.contains("Folder: Cape"));
    assert!(text.contains("Scope: Albedo"));
    assert!(!text.contains("Safe:"));
}

#[test]
fn status_text_within_limits() {
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Very Long Preset Name That Could Be Anything".to_string()),
        folder_name: Some("SomeFolderName".to_string()),
        scope_name: Some("SomeScope".to_string()),
        conflict_count: Some(0),
    };
    let text = generate_status_text(&fields, &crate::services::hotkeys::HotkeyConfig::default());
    assert!(text.lines().count() <= 10);
    assert!(text.len() <= 4096);
}

#[test]
fn write_status_file_atomic() {
    let dir = TempDir::new().unwrap();
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Test".to_string()),
        conflict_count: Some(0),
        ..Default::default()
    };
    let path = write_status_file(
        dir.path(),
        &fields,
        &crate::services::hotkeys::HotkeyConfig::default(),
    )
    .unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Preset: Test"));
}
