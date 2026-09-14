use super::*;

#[test]
fn keybind_text_preserves_key_and_negative_modifier_back_binding() {
    let keybinds = vec![SourceKeyBinding {
        mod_name: "Arlecchino".to_string(),
        keybinds: vec![make_keybinding(
            "KeyToggleBody",
            Some("CTRL+["),
            Some("NO_CTRL+ALT+]"),
        )],
    }];

    let text = generate_keybind_text("Arlecchino", &keybinds, "F7");

    assert!(text.contains("Key: CTRL+["));
    assert!(text.contains("Back: NO_CTRL+ALT+]"));
    assert!(!text.contains("[KeyToggleBody]"));
    assert!(text.contains("[F7] Toggle Overlay"));
}

#[test]
fn keybind_text_shows_section_names_only_when_necessary() {
    let keybinds = vec![SourceKeyBinding {
        mod_name: "Test".to_string(),
        keybinds: vec![
            make_keybinding("KeyToggleBody", Some("1"), None),
            make_keybinding("KeyToggleHead", Some("2"), None),
        ],
    }];

    let text = generate_keybind_text("Test", &keybinds, "F7");

    assert!(text.contains("[KeyToggleBody]"));
    assert!(text.contains("[KeyToggleHead]"));
}

#[test]
fn keybind_text_labels_only_true_toggle_bindings_as_toggle() {
    let mut toggle = make_keybinding("KeyToggle", Some("N"), None);
    toggle.binding_type = Some("toggle".to_string());
    let text = generate_keybind_text(
        "Arlecchino",
        &[SourceKeyBinding {
            mod_name: "Arlecchino".to_string(),
            keybinds: vec![toggle],
        }],
        "F7",
    );

    assert!(text.contains("Toggle: N"));
    assert!(!text.contains("Key: N"));
}

#[test]
fn keybind_text_does_not_fabricate_an_empty_binding_message() {
    let text = generate_keybind_text("Empty", &[], "F7");

    assert!(text.contains("Empty"));
    assert!(!text.contains("No keybinds found"));
    assert!(text.contains("[F7] Toggle Overlay"));
}

#[test]
fn keybind_text_enforces_the_overlay_size_and_line_limits_for_untrusted_text() {
    let mut bindings = Vec::new();
    for index in 0..100 {
        bindings.push(make_keybinding(
            &format!("Key{index}"),
            Some(&"K".repeat(256)),
            Some(&"NO_CTRL+ALT+X".repeat(32)),
        ));
    }
    let text = generate_keybind_text(
        &"Character".repeat(2_000),
        &[SourceKeyBinding {
            mod_name: "Large mod".to_string(),
            keybinds: bindings,
        }],
        &"F7".repeat(128),
    );

    assert!(text.len() <= 8 * 1024);
    assert!(text.lines().count() <= 60);
    assert!(text.contains("... (truncated;"));
    assert!(text.ends_with("..."));
}

#[test]
fn write_keybind_files_creates_one_file_per_character() {
    let dir = TempDir::new().unwrap();
    let matches = vec![make_match_result("Albedo", &["aabb1111", "aabb2222"])];
    let mut keybinds = HashMap::new();
    keybinds.insert(
        "Albedo".to_string(),
        vec![SourceKeyBinding {
            mod_name: "Albedo".to_string(),
            keybinds: vec![make_keybinding("KeyToggle", Some("1"), None)],
        }],
    );

    let written = write_keybind_files(dir.path(), &matches, &keybinds, "F7").unwrap();

    assert_eq!(written.len(), 1);
    assert!(dir.path().join("character_000.txt").exists());
    assert!(!dir.path().join("character_001.txt").exists());
    assert!(!dir.path().join("_fallback.txt").exists());

    let content = std::fs::read_to_string(dir.path().join("character_000.txt")).unwrap();
    assert!(content.contains("Albedo"));
}

#[test]
fn write_keybind_files_with_no_matches_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let written = write_keybind_files(dir.path(), &[], &HashMap::new(), "F7").unwrap();
    assert!(written.is_empty());
}
