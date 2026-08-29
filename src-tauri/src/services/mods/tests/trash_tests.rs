use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn setup_mods() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    fs::create_dir_all(&mods).unwrap();
    (tmp, mods)
}

// Covers: TC-4.5-01 (Delete to system Recycle Bin)
#[test]
fn test_move_to_system_recycle_bin() {
    let (_tmp, mods) = setup_mods();
    let mod_dir = mods.join("Raiden");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("config.ini"), "test").unwrap();

    let result = move_to_trash(&mod_dir);
    assert!(result.is_ok());

    // Original should no longer exist
    assert!(!mod_dir.exists());
}

// Covers: NC-4.5-01 (Source does not exist)
#[test]
fn test_move_to_trash_nonexistent() {
    let (_tmp, _mods) = setup_mods();
    let result = move_to_trash(Path::new("/nonexistent"));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Source does not exist"));
}
