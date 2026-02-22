use super::*;
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
    assert!(!result.unwrap().contains("disabled_"));
}
