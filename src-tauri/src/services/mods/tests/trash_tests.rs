use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_trash() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let mods = tmp.path().join("Mods");
    let trash = tmp.path().join("app_data").join("trash");
    fs::create_dir_all(&mods).unwrap();
    fs::create_dir_all(&trash).unwrap();
    (tmp, mods, trash)
}

// Covers: TC-4.5-01 (Delete to Trash)
#[test]
fn test_move_to_trash_basic() {
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

    // Trash entry should exist
    let trash_entry = trash.join(&meta.id);
    assert!(trash_entry.exists());
    assert!(trash_entry.join("metadata.json").exists());
    assert!(trash_entry.join("Raiden").exists());
}

// Covers: TC-4.5-01 (Restore from Trash)
#[test]
fn test_restore_from_trash() {
    let (_tmp, mods, trash) = setup_trash();
    let mod_dir = mods.join("Ayaka");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("test.txt"), "data").unwrap();

    let meta = move_to_trash(&mod_dir, &trash, None).unwrap();
    assert!(!mod_dir.exists());

    let result = restore_from_trash(&meta.id, &trash, None);
    assert!(result.is_ok());
    assert!(mod_dir.exists());
    assert!(mod_dir.join("test.txt").exists());

    // Trash entry should be cleaned up
    assert!(!trash.join(&meta.id).exists());
}

// Covers: NC-4.5-01 (Source does not exist)
#[test]
fn test_move_to_trash_nonexistent() {
    let (_tmp, _mods, trash) = setup_trash();
    let result = move_to_trash(Path::new("/nonexistent"), &trash, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Source does not exist"));
}

#[test]
fn test_list_trash() {
    let (_tmp, mods, trash) = setup_trash();

    // Create and trash two mods
    let mod1 = mods.join("Mod1");
    let mod2 = mods.join("Mod2");
    fs::create_dir(&mod1).unwrap();
    fs::create_dir(&mod2).unwrap();

    move_to_trash(&mod1, &trash, None).unwrap();
    move_to_trash(&mod2, &trash, None).unwrap();

    let items = list_trash(&trash).unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn test_empty_trash() {
    let (_tmp, mods, trash) = setup_trash();

    let mod1 = mods.join("Mod1");
    fs::create_dir(&mod1).unwrap();
    move_to_trash(&mod1, &trash, None).unwrap();

    let count = empty_trash(&trash).unwrap();
    assert_eq!(count, 1);

    let items = list_trash(&trash).unwrap();
    assert_eq!(items.len(), 0);
}

#[test]
fn test_restore_conflict() {
    let (_tmp, mods, trash) = setup_trash();
    let mod_dir = mods.join("Conflict");
    fs::create_dir(&mod_dir).unwrap();

    let meta = move_to_trash(&mod_dir, &trash, None).unwrap();

    // Re-create the original folder
    fs::create_dir(&mod_dir).unwrap();

    let result = restore_from_trash(&meta.id, &trash, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn test_restore_context_mismatch() {
    let (_tmp, mods, trash) = setup_trash();
    let mod_dir = mods.join("Mismatch");
    fs::create_dir(&mod_dir).unwrap();

    let game1 = "game1".to_string();
    let game2 = "game2".to_string();
    let meta = move_to_trash(&mod_dir, &trash, Some(game1)).unwrap();

    let result = restore_from_trash(&meta.id, &trash, Some(&game2));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Context mismatch"));
}
