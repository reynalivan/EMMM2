use super::*;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
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

#[test]
fn prepared_trash_move_rejects_replaced_source_identity() {
    let (_tmp, mods) = setup_mods();
    let mod_dir = mods.join("Raiden");
    let displaced = mods.join("Raiden-original");
    fs::create_dir(&mod_dir).unwrap();
    let prepared = prepare_trash_move(&mod_dir).unwrap();

    fs::rename(&mod_dir, &displaced).unwrap();
    fs::create_dir(&mod_dir).unwrap();

    let error = prepared.execute(&WatcherState::new()).unwrap_err();
    assert!(error.to_string().contains("changed after validation"));
    assert!(mod_dir.exists());
    assert!(displaced.exists());
    assert!(!prepared.quarantine().exists());
}

#[test]
fn prepared_trash_move_rejects_external_destination_collision() {
    let (_tmp, mods) = setup_mods();
    let mod_dir = mods.join("Raiden");
    fs::create_dir(&mod_dir).unwrap();
    let prepared = prepare_trash_move(&mod_dir).unwrap();
    fs::create_dir(prepared.quarantine()).unwrap();

    let error = prepared.execute(&WatcherState::new()).unwrap_err();
    assert!(error.to_string().contains("destination already exists"));
    assert!(mod_dir.exists());
    assert!(prepared.quarantine().exists());
}
