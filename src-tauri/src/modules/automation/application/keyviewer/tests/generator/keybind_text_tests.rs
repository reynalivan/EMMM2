use super::*;

// ─── Keybind Text Generation ─────────────────────────────────────────────────

#[test]
fn keybind_text_with_key_and_back() {
    let kbs = vec![SourceKeyBinding {
        mod_name: "Albedo".to_string(),
        keybinds: vec![make_keybinding("KeyToggleBody", Some("1"), Some("2"))],
    }];
    let text = generate_keybind_text("Albedo", &kbs, "F7");
    assert!(text.contains("Albedo"));
    assert!(text.contains("Key: 1"));
    assert!(text.contains("Back: 2"));
}

#[test]
fn keybind_text_key_only() {
    let kbs = vec![SourceKeyBinding {
        mod_name: "Amber".to_string(),
        keybinds: vec![make_keybinding("KeyToggleBody", Some("3"), None)],
    }];
    let text = generate_keybind_text("Amber", &kbs, "F7");
    assert!(text.contains("Key: 3"));
    assert!(!text.contains("Back:"));
}

#[test]
fn keybind_text_no_keybinds() {
    let text = generate_keybind_text("Empty", &[], "F7");
    assert!(text.contains("No keybinds found"));
}

#[test]
fn keybind_text_multiple_keybinds() {
    let kbs = vec![SourceKeyBinding {
        mod_name: "Test".to_string(),
        keybinds: vec![
            make_keybinding("KeyToggleBody", Some("1"), None),
            make_keybinding("KeyToggleHead", Some("2"), None),
        ],
    }];
    let text = generate_keybind_text("Test", &kbs, "F7");
    assert!(text.contains("[KeyToggleBody]"));
    assert!(text.contains("[KeyToggleHead]"));
}

// ─── Write Keybind Files ─────────────────────────────────────────────────────

#[test]
fn write_keybind_files_creates_per_sentinel() {
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

    assert_eq!(written.len(), 2);
    assert!(dir.path().join("aabb1111.txt").exists());
    assert!(dir.path().join("aabb2222.txt").exists());
    assert!(!dir.path().join("_fallback.txt").exists());

    let content = std::fs::read_to_string(dir.path().join("aabb1111.txt")).unwrap();
    assert!(content.contains("Albedo"));
}

#[test]
fn write_keybind_files_with_no_matches_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let written = write_keybind_files(dir.path(), &[], &HashMap::new(), "F7").unwrap();
    assert!(written.is_empty());
}
