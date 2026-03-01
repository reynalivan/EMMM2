use crate::services::explorer::listing::{build_mod_folder_from_fs_entry, scan_fs_folders};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_scan_fs_folders_empty() {
    let temp_dir = TempDir::new().unwrap();
    let result = scan_fs_folders(temp_dir.path(), temp_dir.path(), None)
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_scan_fs_folders_with_mods() {
    let temp_dir = TempDir::new().unwrap();
    let mods_path = temp_dir.path();

    // Create an enabled mod folder
    let mod1 = mods_path.join("mod1");
    fs::create_dir(&mod1).unwrap();
    fs::write(mod1.join("mod.ini"), "[TextureOverride]\n").unwrap(); // Make it a ModPackRoot

    // Create a disabled mod folder
    let mod2 = mods_path.join("DISABLED mod2");
    fs::create_dir(&mod2).unwrap();

    // Create a hidden folder
    let hidden = mods_path.join(".hidden");
    fs::create_dir(&hidden).unwrap();

    // Create a file (should be ignored)
    fs::write(mods_path.join("some_file.txt"), "data").unwrap();

    let result = scan_fs_folders(mods_path, mods_path, None).await.unwrap();

    // Should only find the two directories, hidden and files are ignored.
    // They are sorted alphabetically by display name: "mod1", "mod2"
    assert_eq!(result.len(), 2);

    assert_eq!(result[0].name, "mod1");
    assert_eq!(result[0].folder_name, "mod1");
    assert!(result[0].is_enabled);
    assert_eq!(result[0].node_type, "FlatModRoot");

    assert_eq!(result[1].name, "mod2"); // Display name normalized
    assert_eq!(result[1].folder_name, "DISABLED mod2");
    assert!(!result[1].is_enabled);
}

#[test]
fn test_build_mod_folder_from_fs_entry() {
    let temp_dir = TempDir::new().unwrap();

    let mod_dir = temp_dir.path().join("my_mod");
    fs::create_dir(&mod_dir).unwrap();

    let entry = fs::read_dir(temp_dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap();

    let folder = build_mod_folder_from_fs_entry(entry, None).unwrap();
    assert_eq!(folder.name, "my_mod");
    assert_eq!(folder.folder_name, "my_mod");
    assert!(folder.is_enabled);
}
