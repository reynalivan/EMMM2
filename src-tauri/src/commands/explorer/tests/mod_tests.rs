use crate::commands::explorer::listing::list_mod_folders_inner;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_list_mod_folders_basic() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    fs::create_dir(&mods).unwrap();
    fs::create_dir(mods.join("Raiden")).unwrap();
    fs::create_dir(mods.join("DISABLED Ayaka")).unwrap();
    fs::create_dir(mods.join("Albedo")).unwrap();

    let result = list_mod_folders_inner(mods.to_string_lossy().to_string(), None).await;
    assert!(result.is_ok());

    let folders = result.unwrap();
    assert_eq!(folders.len(), 3);

    // Sorted alphabetically by display_name
    assert_eq!(folders[0].name, "Albedo");
    assert!(folders[0].is_enabled);
    assert!(folders[0].is_directory);

    assert_eq!(folders[1].name, "Ayaka");
    assert!(!folders[1].is_enabled);
    assert_eq!(folders[1].folder_name, "DISABLED Ayaka");

    assert_eq!(folders[2].name, "Raiden");
    assert!(folders[2].is_enabled);
}

#[tokio::test]
async fn test_list_mod_folders_skips_files_and_hidden() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    fs::create_dir(&mods).unwrap();
    fs::create_dir(mods.join("ValidMod")).unwrap();
    fs::create_dir(mods.join(".hidden")).unwrap();
    fs::write(mods.join("readme.txt"), "hello").unwrap();

    let result = list_mod_folders_inner(mods.to_string_lossy().to_string(), None).await;
    let folders = result.unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0].name, "ValidMod");
}

#[tokio::test]
async fn test_list_mod_folders_nonexistent_path() {
    let result = list_mod_folders_inner("C:\\nonexistent\\fake\\path".to_string(), None).await;
    // Base path validation still returns Err
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_list_mod_folders_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    fs::create_dir(&mods).unwrap();

    let result = list_mod_folders_inner(mods.to_string_lossy().to_string(), None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

// Covers: TC-4.1-01 (Deep Navigation)
#[tokio::test]
async fn test_list_mod_folders_deep_navigation() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    let raiden = mods.join("Raiden");
    let set1 = raiden.join("Set1");
    fs::create_dir_all(&set1).unwrap();
    fs::create_dir(raiden.join("Set2")).unwrap();

    let result = list_mod_folders_inner(
        mods.to_string_lossy().to_string(),
        Some("Raiden".to_string()),
    )
    .await;
    assert!(result.is_ok());

    let folders = result.unwrap();
    assert_eq!(folders.len(), 2);
    assert_eq!(folders[0].name, "Set1");
    assert_eq!(folders[1].name, "Set2");
}

// Covers: TC-4.1-01 (Deep Navigation — invalid sub_path)
#[tokio::test]
async fn test_list_mod_folders_invalid_subpath() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    fs::create_dir(&mods).unwrap();

    let result = list_mod_folders_inner(
        mods.to_string_lossy().to_string(),
        Some("NonExistent".to_string()),
    )
    .await;
    // With Filesystem Truth design, missing subpaths just return empty Ok vectors
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

// Covers: TC-4.2-02 (thumbnail resolved lazily via get_mod_thumbnail)
#[tokio::test]
async fn test_list_mod_folders_thumbnail_deferred() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    let mod_folder = mods.join("Raiden");
    fs::create_dir_all(&mod_folder).unwrap();
    fs::write(mod_folder.join("preview.png"), "fake png data").unwrap();

    let result = list_mod_folders_inner(mods.to_string_lossy().to_string(), None).await;
    let folders = result.unwrap();
    assert_eq!(folders.len(), 1);
    assert!(folders[0].thumbnail_path.is_none());
}

// Covers: DI-4.03 (info.json detection)
#[tokio::test]
async fn test_list_mod_folders_info_json_detection() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    let with_info = mods.join("WithInfo");
    let without_info = mods.join("NoInfo");
    fs::create_dir_all(&with_info).unwrap();
    fs::create_dir_all(&without_info).unwrap();
    fs::write(with_info.join("info.json"), "{}").unwrap();

    let result = list_mod_folders_inner(mods.to_string_lossy().to_string(), None).await;
    let folders = result.unwrap();
    assert_eq!(folders.len(), 2);

    let info_folder = folders.iter().find(|f| f.name == "NoInfo").unwrap();
    assert!(!info_folder.has_info_json);

    let no_info_folder = folders.iter().find(|f| f.name == "WithInfo").unwrap();
    assert!(no_info_folder.has_info_json);
}

// Covers: TC-4.1-02 (modified_at is populated)
#[tokio::test]
async fn test_list_mod_folders_has_modified_at() {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    fs::create_dir_all(mods.join("TestMod")).unwrap();

    let result = list_mod_folders_inner(mods.to_string_lossy().to_string(), None).await;
    let folders = result.unwrap();
    assert_eq!(folders.len(), 1);
    assert!(folders[0].modified_at > 0);
}
