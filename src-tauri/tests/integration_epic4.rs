use emmm2_lib::commands::mod_cmds;
use emmm2_lib::services::file_ops::trash;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_epic4_full_maintenance_flow() {
    use emmm2_lib::services::watcher::WatcherState;

    // -------------------------------------------------------------------------
    // Setup: Create a mock environment
    // -------------------------------------------------------------------------
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let mods_dir = root.join("Mods");
    let trash_dir = root.join("Trash");

    fs::create_dir(&mods_dir).unwrap();
    fs::create_dir(&trash_dir).unwrap();

    // Create an initial mod: "Raiden"
    let mod_path = mods_dir.join("Raiden");
    fs::create_dir(&mod_path).unwrap();
    fs::write(mod_path.join("README.txt"), "Original Content").unwrap();

    println!("Created mod at: {:?}", mod_path);

    // Instantiate WatcherState
    let state = WatcherState::new();

    // -------------------------------------------------------------------------
    // Step 1: Rename "Raiden" -> "Shogun"
    // -------------------------------------------------------------------------
    let rename_result = mod_cmds::rename_mod_folder_inner(
        &state,
        mod_path.to_string_lossy().to_string(),
        "Shogun".to_string(),
    )
    .await
    .expect("Rename should succeed");

    assert_eq!(rename_result.new_name, "Shogun");
    assert!(!mod_path.exists(), "Old path should not exist");

    let shogun_path = mods_dir.join("Shogun");
    assert!(shogun_path.exists(), "New path should exist");

    println!("Step 1 (Rename) Passed");

    // -------------------------------------------------------------------------
    // Step 2: Toggle (Disable) "Shogun" -> "DISABLED Shogun"
    // -------------------------------------------------------------------------
    let toggle_result = mod_cmds::toggle_mod_inner(
        &state,
        shogun_path.to_string_lossy().to_string(),
        false, // enable = false => disable
    )
    .await
    .expect("Toggle disable should succeed");

    assert!(toggle_result.contains("DISABLED Shogun"));

    let disabled_path = mods_dir.join("DISABLED Shogun");
    assert!(disabled_path.exists(), "Disabled path should exist");
    assert!(!shogun_path.exists(), "Enabled path should be gone");

    println!("Step 2 (Toggle) Passed");

    // -------------------------------------------------------------------------
    // Step 3: Delete to Trash
    // NOTE: calling service directly as command requires AppHandle
    // -------------------------------------------------------------------------
    let trash_meta =
        trash::move_to_trash(&disabled_path, &trash_dir, Some("game_id_test".to_string()))
            .expect("Move to trash should succeed");

    assert!(!disabled_path.exists(), "File should be gone from mods dir");
    assert!(
        trash_dir.join(&trash_meta.id).exists(),
        "File should be in trash dir"
    );

    println!("Step 3 (Delete) Passed");

    // -------------------------------------------------------------------------
    // Step 4: Restore from Trash
    // -------------------------------------------------------------------------
    let restored_path_str =
        trash::restore_from_trash(&trash_meta.id, &trash_dir).expect("Restore should succeed");

    let restored_path = std::path::PathBuf::from(restored_path_str);
    assert_eq!(
        restored_path, disabled_path,
        "Should restore to the Disabled path (state preserved)"
    );
    assert!(disabled_path.exists(), "Restored file should exist");
    assert!(
        !trash_dir.join(&trash_meta.id).exists(),
        "Trash entry should be gone"
    );

    println!("Step 4 (Restore) Passed");
}
