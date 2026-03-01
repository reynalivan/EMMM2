use super::*;
use crate::services::scanner::watcher::WatcherState;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_bulk_toggle_partial_failure() {
    let tmp = TempDir::new().unwrap();
    let mod1_dir = tmp.path().join("Mod1");
    let mod2_dir = tmp.path().join("Mod2"); // Will not exist

    fs::create_dir(&mod1_dir).unwrap();

    let state = WatcherState::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Convert to disabled
    let paths = vec![
        mod1_dir.to_string_lossy().to_string(),
        mod2_dir.to_string_lossy().to_string(),
    ];

    let result = rt.block_on(bulk_toggle_mods_inner(&state, paths, false));

    assert!(result.is_ok());
    let bulk_res = result.unwrap();

    // 1 success, 1 failure
    assert_eq!(bulk_res.success.len(), 1);
    assert_eq!(bulk_res.failures.len(), 1);

    assert!(tmp.path().join("DISABLED Mod1").exists());
}

#[test]
fn test_bulk_delete_moves_to_trash() {
    let tmp = TempDir::new().unwrap();
    let mod1_dir = tmp.path().join("ModOne");
    let mod2_dir = tmp.path().join("ModTwo");

    fs::create_dir(&mod1_dir).unwrap();
    fs::create_dir(&mod2_dir).unwrap();

    let trash_dir = tmp.path().join("trash");
    fs::create_dir(&trash_dir).unwrap();

    let state = WatcherState::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let paths = vec![
        mod1_dir.to_string_lossy().to_string(),
        mod2_dir.to_string_lossy().to_string(),
    ];

    let result = rt.block_on(bulk_delete_mods_inner(&state, &trash_dir, paths, None));

    assert!(result.is_ok());
    let bulk_res = result.unwrap();

    assert_eq!(bulk_res.success.len(), 2);
    assert_eq!(bulk_res.failures.len(), 0);

    assert!(!mod1_dir.exists());
    assert!(!mod2_dir.exists());
    // In delete_mod_inner, it moves to trash_dir with a timestamp.
    // Instead of checking exact name, verify that trash_dir has 2 items.
    let trash_items = fs::read_dir(&trash_dir).unwrap().count();
    assert_eq!(trash_items, 2);
}
