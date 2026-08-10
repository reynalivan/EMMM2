use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn setup_trash() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    let trash = tmp.path().join("app_data").join("trash");
    fs::create_dir_all(&mods).unwrap();
    fs::create_dir_all(&trash).unwrap();
    (tmp, mods, trash)
}

// Covers: TC-4.5-01 (Delete to system Recycle Bin)
#[test]
fn test_move_to_system_recycle_bin() {
    let (_tmp, mods, trash) = setup_trash();
    let mod_dir = mods.join("Raiden");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("config.ini"), "test").unwrap();

    let result = move_to_trash(&mod_dir, &trash, Some("game1".to_string()));
    assert!(result.is_ok());

    let meta = result.unwrap();
    assert_eq!(meta.original_name, "Raiden");
    assert!(meta.original_path.contains("Raiden"));
    assert_eq!(meta.game_id, Some("game1".to_string()));

    // Original should no longer exist
    assert!(!mod_dir.exists());

    assert!(!trash.join(&meta.id).exists());
}

#[test]
fn legacy_trash_commands_do_not_manage_user_files() {
    let (_tmp, _mods, trash) = setup_trash();
    assert!(list_trash(&trash).unwrap().is_empty());
    assert!(empty_trash(&trash).is_err());
    assert!(restore_from_trash("legacy", &trash, None).is_err());
}

// Covers: NC-4.5-01 (Source does not exist)
#[test]
fn test_move_to_trash_nonexistent() {
    let (_tmp, _mods, trash) = setup_trash();
    let result = move_to_trash(Path::new("/nonexistent"), &trash, None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Source does not exist"));
}
