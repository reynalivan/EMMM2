use emmm_lib::modules::library::adapters::inbound::mod_core_cmds;
use emmm_lib::modules::library::application::mods::trash;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_epic4_full_maintenance_flow() {
    use emmm_lib::modules::workspace::application::scanner::watcher::WatcherState;

    // -------------------------------------------------------------------------
    // Setup: Create a mock environment
    // -------------------------------------------------------------------------
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let mods_dir = root.join("Mods");

    fs::create_dir(&mods_dir).unwrap();

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
    let rename_result = mod_core_cmds::rename_mod_folder_inner(
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
    let toggle_result = mod_core_cmds::toggle_mod_inner(
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
    trash::move_to_trash(&disabled_path).expect("Move to trash should succeed");

    assert!(!disabled_path.exists(), "File should be gone from mods dir");

    println!("Step 3 (Delete) Passed");
}
