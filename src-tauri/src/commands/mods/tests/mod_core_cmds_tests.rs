use super::*;
use crate::services::mods::trash;
use std::fs;
use tempfile::TempDir;

#[test]
fn check_folder_contents_non_empty() {
    let tmp = TempDir::new().unwrap();
    let folder = tmp.path().join("TestMod");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("file1.ini"), "data").unwrap();
    fs::write(folder.join("file2.buf"), "data").unwrap();
    fs::create_dir(folder.join("subfolder")).unwrap();

    let info = check_folder_contents(&folder).unwrap();

    assert_eq!(info.name, "TestMod");
    assert_eq!(info.item_count, 3);
    assert!(!info.is_empty);
}

#[test]
fn check_folder_contents_empty() {
    let tmp = TempDir::new().unwrap();
    let folder = tmp.path().join("EmptyMod");
    fs::create_dir(&folder).unwrap();
    let info = check_folder_contents(&folder).unwrap();
    assert_eq!(info.item_count, 0);
    assert!(info.is_empty);
}

#[test]
fn rename_rejects_case_insensitive_duplicate() {
    let tmp = TempDir::new().unwrap();
    let raiden = tmp.path().join("Raiden");
    fs::create_dir(&raiden).unwrap();
    let other = tmp.path().join("Other");
    fs::create_dir(&other).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(rename_mod_folder_inner(
        &state,
        other.to_string_lossy().to_string(),
        "raiden".to_string(),
    ));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn standardize_prefix_lowercase_underscore() {
    assert_eq!(standardize_prefix("disabled_Ayaka", true), "Ayaka");
    assert_eq!(
        standardize_prefix("disabled_Ayaka", false),
        "DISABLED Ayaka"
    );
}

#[test]
fn standardize_prefix_dash_variant() {
    assert_eq!(standardize_prefix("DISABLED-Keqing", true), "Keqing");
    assert_eq!(
        standardize_prefix("DISABLED-Keqing", false),
        "DISABLED Keqing"
    );
}

#[test]
fn toggle_with_bad_prefix_filesystem() {
    let tmp = TempDir::new().unwrap();
    let bad_name = tmp.path().join("disabled_Ayaka");
    fs::create_dir(&bad_name).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(toggle_mod_inner(
        &state,
        bad_name.to_string_lossy().to_string(),
        true,
    ));

    assert!(result.is_ok());
    assert!(result.is_ok());
    assert!(!result.unwrap().contains("disabled_"));
}

#[test]
fn rename_rejects_invalid_chars() {
    let tmp = TempDir::new().unwrap();
    let old_dir = tmp.path().join("OldMod");
    fs::create_dir(&old_dir).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(rename_mod_folder_inner(
        &state,
        old_dir.to_string_lossy().to_string(),
        "New*Mod".to_string(),
    ));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid folder name"));
}

#[test]
fn delete_mod_moves_to_trash() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("DeleteMe");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("test.txt"), "hello").unwrap();

    let trash_dir = tmp.path().join("trash");
    fs::create_dir(&trash_dir).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(trash::move_to_trash_guarded(
        &state,
        &trash_dir,
        mod_dir.to_string_lossy().to_string(),
        None,
    ));

    assert!(result.is_ok());
    assert!(
        !mod_dir.exists(),
        "Original directory should be deleted/moved"
    );
}

#[test]
fn rename_enabled_mod_success() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("EnabledMod");
    fs::create_dir(&mod_dir).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(rename_mod_folder_inner(
        &state,
        mod_dir.to_string_lossy().to_string(),
        "BetterName".to_string(),
    ));

    assert!(result.is_ok());
    let res = result.unwrap();
    assert_eq!(res.new_name, "BetterName");
    assert!(res.new_path.ends_with("BetterName"));
    assert!(Path::new(&res.new_path).exists());
    assert!(!mod_dir.exists());
}

#[test]
fn rename_disabled_mod_preserves_prefix() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("DISABLED OldName");
    fs::create_dir(&mod_dir).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(rename_mod_folder_inner(
        &state,
        mod_dir.to_string_lossy().to_string(),
        "NewName".to_string(),
    ));

    assert!(result.is_ok());
    let res = result.unwrap();
    assert_eq!(res.new_name, "NewName");
    assert!(Path::new(&res.new_path).exists());
    assert!(res.new_path.ends_with("DISABLED NewName"));
}

#[test]
fn rename_path_collision() {
    let tmp = TempDir::new().unwrap();
    let mod_a = tmp.path().join("ModA");
    let mod_b = tmp.path().join("ModB");
    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(rename_mod_folder_inner(
        &state,
        mod_b.to_string_lossy().to_string(),
        "ModA".to_string(),
    ));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

// Covers: TC-20-05 (Collision detection on toggle)
#[test]
fn toggle_mod_collision() {
    let tmp = TempDir::new().unwrap();
    let disabled_mod = tmp.path().join("DISABLED MyMod");
    let enabled_mod = tmp.path().join("MyMod");

    fs::create_dir(&disabled_mod).unwrap();
    fs::create_dir(&enabled_mod).unwrap(); // The conflict existing

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();

    let result = rt.block_on(toggle_mod_inner(
        &state,
        disabled_mod.to_string_lossy().to_string(),
        true, // attempt to enable "DISABLED MyMod" -> "MyMod"
    ));

    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Conflict") || err_msg.contains("exists"));
}

#[test]
fn rename_updates_info_json_preserves_other_keys() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("InfoMod");
    fs::create_dir(&mod_dir).unwrap();

    // Create an initial info.json with some extra fields
    let initial_info = r#"{
        "actual_name": "OldName",
        "author": "TestAuthor",
        "description": "Don't touch this",
        "version": "1.0",
        "tags": ["tag1"],
        "is_safe": true,
        "is_favorite": true,
        "is_auto_sync": false,
        "preset_name": [],
        "metadata": {}
    }"#;
    fs::write(mod_dir.join("info.json"), initial_info).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = WatcherState::new();
    let result = rt.block_on(rename_mod_folder_inner(
        &state,
        mod_dir.to_string_lossy().to_string(),
        "NewName".to_string(),
    ));

    assert!(result.is_ok());
    let res = result.unwrap();
    let new_path = Path::new(&res.new_path);
    assert!(new_path.exists());

    // Read the updated info.json
    let info_content = fs::read_to_string(new_path.join("info.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&info_content).unwrap();

    // Verify name changed but others preserved
    assert_eq!(parsed["actual_name"], "NewName");
    assert_eq!(parsed["author"], "TestAuthor");
    assert_eq!(parsed["description"], "Don't touch this");
    assert_eq!(parsed["is_favorite"], true);
}
