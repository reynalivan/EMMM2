use super::*;
use tempfile::TempDir;

// Covers: DI-4.03 (info.json created with defaults)
#[test]
fn test_create_default_info_json() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("Raiden_Outfit_v2");
    fs::create_dir(&mod_dir).unwrap();

    let info = create_default_info_json(&mod_dir).unwrap();
    assert_eq!(info.actual_name, "Raiden_Outfit_v2");
    assert_eq!(info.author, "Unknown");
    assert_eq!(info.version, "1.0");
    assert!(info.is_safe);
    assert!(!info.is_favorite);

    // File should exist
    assert!(mod_dir.join("info.json").exists());
}

// Covers: DI-4.03 (strips DISABLED prefix)
#[test]
fn test_create_default_strips_disabled_prefix() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("DISABLED Ayaka_Skin");
    fs::create_dir(&mod_dir).unwrap();

    let info = create_default_info_json(&mod_dir).unwrap();
    assert_eq!(info.actual_name, "Ayaka_Skin");
}

#[test]
fn test_read_info_json_nonexistent() {
    let tmp = TempDir::new().unwrap();
    let result = read_info_json(tmp.path()).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_read_info_json_valid() {
    let tmp = TempDir::new().unwrap();
    let json = r#"{"actual_name": "Test", "author": "Me", "tags": ["cool"]}"#;
    fs::write(tmp.path().join("info.json"), json).unwrap();

    let info = read_info_json(tmp.path()).unwrap().unwrap();
    assert_eq!(info.actual_name, "Test");
    assert_eq!(info.author, "Me");
    assert_eq!(info.tags, vec!["cool"]);
    // Defaults applied
    assert_eq!(info.version, "1.0");
    assert!(info.is_safe);
}

#[test]
fn test_read_info_json_malformed() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("info.json"), "not json at all").unwrap();

    let result = read_info_json(tmp.path());
    assert!(result.is_err());
}

// Covers: DI-4.03 (partial update merges, not overwrites)
#[test]
fn test_update_info_json_merge() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("TestMod");
    fs::create_dir(&mod_dir).unwrap();

    // Create initial
    create_default_info_json(&mod_dir).unwrap();

    // Update only tags and is_favorite
    let update = ModInfoUpdate {
        tags: Some(vec!["nsfw".to_string(), "outfit".to_string()]),
        is_favorite: Some(true),
        ..Default::default()
    };

    let result = update_info_json(&mod_dir, &update).unwrap();
    assert_eq!(result.tags, vec!["nsfw", "outfit"]);
    assert!(result.is_favorite);
    // Original fields preserved
    assert_eq!(result.actual_name, "TestMod");
    assert_eq!(result.author, "Unknown");
}

// Covers: EC-4.06 (Orphaned info.json — empty folder)
#[test]
fn test_read_info_json_with_empty_object() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("info.json"), "{}").unwrap();

    let info = read_info_json(tmp.path()).unwrap().unwrap();
    // Defaults should be applied for missing fields
    assert_eq!(info.actual_name, "");
    assert_eq!(info.author, "Unknown");
    assert_eq!(info.version, "1.0");
    assert!(info.is_safe);
}

#[test]
fn test_create_does_not_overwrite() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModX");
    fs::create_dir(&mod_dir).unwrap();

    // Create custom info.json first
    let custom = r#"{"actual_name": "Custom Name", "author": "Cool Author"}"#;
    fs::write(mod_dir.join("info.json"), custom).unwrap();

    // create_default should return existing, not overwrite
    let info = create_default_info_json(&mod_dir).unwrap();
    assert_eq!(info.actual_name, "Custom Name");
    assert_eq!(info.author, "Cool Author");
}

// Covers: update creates info.json if it doesn't exist
#[test]
fn test_update_creates_if_missing() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("NewMod");
    fs::create_dir(&mod_dir).unwrap();

    let update = ModInfoUpdate {
        author: Some("New Author".to_string()),
        ..Default::default()
    };

    let result = update_info_json(&mod_dir, &update).unwrap();
    assert_eq!(result.author, "New Author");
    assert_eq!(result.actual_name, "NewMod");
    assert!(mod_dir.join("info.json").exists());
}
