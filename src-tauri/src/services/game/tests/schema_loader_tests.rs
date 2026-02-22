use super::*;
use std::io::Write;
use tempfile::TempDir;

// Covers: NC-3.4-02 (Schema Load Failure → fallback)
#[test]
fn test_default_schema_has_four_categories() {
    let schema = default_schema();
    assert_eq!(schema.categories.len(), 4);
    assert_eq!(schema.categories[0].name, "Character");
    assert_eq!(schema.categories[1].name, "Weapon");
    assert_eq!(schema.categories[2].name, "UI");
    assert_eq!(schema.categories[3].name, "Other");
    assert!(schema.filters.is_empty());
}

// Covers: NC-3.4-02 (Schema file missing → fallback)
#[test]
fn test_load_schema_missing_file_returns_default() {
    let temp = TempDir::new().unwrap();
    let schema = load_schema(temp.path(), "GIMI");
    assert_eq!(schema.categories.len(), 4);
    assert_eq!(schema.categories[0].name, "Character");
}

// Covers: NC-3.4-02 (Schema corrupt → fallback)
#[test]
fn test_load_schema_corrupt_json_returns_default() {
    let temp = TempDir::new().unwrap();
    let schemas_dir = temp.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    let mut file = std::fs::File::create(schemas_dir.join("gimi.json")).unwrap();
    file.write_all(b"{ invalid json !!!").unwrap();

    let schema = load_schema(temp.path(), "GIMI");
    assert_eq!(schema.categories.len(), 4);
}

// Covers: DI-3.02 (Schema with empty categories → fallback)
#[test]
fn test_load_schema_empty_categories_returns_default() {
    let temp = TempDir::new().unwrap();
    let schemas_dir = temp.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    let mut file = std::fs::File::create(schemas_dir.join("gimi.json")).unwrap();
    file.write_all(b"{\"categories\": [], \"filters\": []}")
        .unwrap();

    let schema = load_schema(temp.path(), "GIMI");
    assert_eq!(schema.categories.len(), 4, "Should fallback on empty");
}

// Covers: TC-3.4 (Valid schema loads correctly)
#[test]
fn test_load_schema_valid_json_returns_parsed() {
    let temp = TempDir::new().unwrap();
    let schemas_dir = temp.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    let valid_json = r#"{
        "categories": [
            { "name": "Resonator", "icon": "User", "color": "primary" },
            { "name": "Weapon", "icon": "Sword", "color": "secondary" }
        ],
        "filters": [
            { "key": "element", "label": "Element", "options": ["Spectro", "Havoc"] }
        ]
    }"#;

    let mut file = std::fs::File::create(schemas_dir.join("wwmi.json")).unwrap();
    file.write_all(valid_json.as_bytes()).unwrap();

    let schema = load_schema(temp.path(), "WWMI");
    assert_eq!(schema.categories.len(), 2);
    assert_eq!(schema.categories[0].name, "Resonator");
    assert_eq!(schema.filters.len(), 1);
    assert_eq!(schema.filters[0].key, "element");
    assert_eq!(schema.filters[0].options.len(), 2);
}

// Covers: TC-3.1-01 (case insensitive file lookup)
#[test]
fn test_load_schema_case_insensitive_game_type() {
    let temp = TempDir::new().unwrap();
    let schemas_dir = temp.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    let valid =
        r#"{"categories": [{"name": "Test", "icon": "X", "color": "info"}], "filters": []}"#;
    let mut file = std::fs::File::create(schemas_dir.join("srmi.json")).unwrap();
    file.write_all(valid.as_bytes()).unwrap();

    // Should find srmi.json even when called with "SRMI"
    let schema = load_schema(temp.path(), "SRMI");
    assert_eq!(schema.categories.len(), 1);
    assert_eq!(schema.categories[0].name, "Test");
}

// Covers: normalize_game_type maps legacy names to canonical XXMI codes
#[test]
fn test_normalize_game_type() {
    assert_eq!(normalize_game_type("StarRail"), "srmi");
    assert_eq!(normalize_game_type("SRMI"), "srmi");
    assert_eq!(normalize_game_type("Genshin"), "gimi");
    assert_eq!(normalize_game_type("GIMI"), "gimi");
    assert_eq!(normalize_game_type("ZZZ"), "zzmi");
    assert_eq!(normalize_game_type("Wuthering"), "wwmi");
    assert_eq!(normalize_game_type("Endfield"), "efmi");
    // Unknown passthrough
    assert_eq!(normalize_game_type("CustomGame"), "customgame");
}

// Covers: load_schema with legacy game_type resolves to correct schema
#[test]
fn test_load_schema_with_legacy_game_type() {
    let temp = TempDir::new().unwrap();
    let schemas_dir = temp.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    let valid = r#"{"categories": [{"name": "Character", "icon": "User", "color": "primary", "filters": [{"key": "element", "label": "Element", "options": ["Fire"]}]}], "filters": []}"#;
    let mut file = std::fs::File::create(schemas_dir.join("srmi.json")).unwrap();
    file.write_all(valid.as_bytes()).unwrap();

    // "StarRail" (legacy) should normalize to "srmi" and find srmi.json
    let schema = load_schema(temp.path(), "StarRail");
    assert_eq!(schema.categories.len(), 1);
    assert_eq!(schema.categories[0].name, "Character");
    // Per-category filters should be present
    let filters = schema.categories[0].filters.as_ref().unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].key, "element");
}
