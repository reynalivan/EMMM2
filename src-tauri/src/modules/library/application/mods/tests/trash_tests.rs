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

#[test]
fn canary_startup_cleanup_refuses_replaced_root_or_quarantine() {
    use super::service::finalize_recorded_quarantines_with;
    use std::cell::Cell;
    let (_tmp, mods) = setup_mods();
    let source = mods.join("Mod");
    fs::create_dir(&source).unwrap();
    let prepared = prepare_trash_move(&source).unwrap();
    let expected_root_identity = crate::platform::fs::file_utils::filesystem_identity(&mods);
    prepared.execute(&WatcherState::new()).unwrap();
    let mut record = RecordedQuarantine {
        path: prepared.quarantine().to_path_buf(),
        source_path: Some(source),
        game_root: mods.clone(),
        expected_root_identity,
        expected_identity: Some(prepared.expected_identity().to_string()),
    };
    let calls = Cell::new(0);
    let original_path = record.path.clone();
    let sink = |path: &Path| {
        calls.set(calls.get() + 1);
        fs::rename(path, &original_path).map_err(crate::shared::errors::AppError::from)
    };
    assert!(finalize_recorded_quarantines_with([record.clone()], &sink).is_empty());
    assert_eq!(calls.get(), 1);
    calls.set(0);
    let retained = mods.join("owned-quarantine");
    fs::rename(&record.path, &retained).unwrap();
    fs::create_dir(&record.path).unwrap();
    assert_eq!(
        finalize_recorded_quarantines_with([record.clone()], &sink).len(),
        1
    );
    assert_eq!(calls.get(), 0);
    assert!(record.path.exists());
    record.expected_identity = crate::platform::fs::file_utils::filesystem_identity(&record.path);
    record.expected_root_identity = Some("foreign-root".to_string());
    assert_eq!(
        finalize_recorded_quarantines_with([record.clone()], &sink).len(),
        1
    );
    assert_eq!(calls.get(), 0);
    record.expected_root_identity = None;
    assert_eq!(finalize_recorded_quarantines_with([record], &sink).len(), 1);
    assert_eq!(calls.get(), 0);
    assert!(retained.exists());
}

#[test]
fn cleanup_handoff_does_not_recycle_a_recreated_recorded_path() {
    use super::service::finalize_recorded_quarantines_with;
    let (_root, mods) = setup_mods();
    let source = mods.join("Mod");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("owned.txt"), "owned payload").unwrap();
    let prepared = prepare_trash_move(&source).unwrap();
    prepared.execute(&WatcherState::new()).unwrap();
    let record = RecordedQuarantine {
        path: prepared.quarantine().to_path_buf(),
        source_path: Some(source),
        game_root: mods.clone(),
        expected_root_identity: crate::platform::fs::file_utils::filesystem_identity(&mods),
        expected_identity: Some(prepared.expected_identity().to_string()),
    };
    let original = record.path.clone();
    let expected_identity = record.expected_identity.clone();
    let warnings = finalize_recorded_quarantines_with([record], &|payload| {
        assert_ne!(payload, original);
        assert_eq!(
            crate::platform::fs::file_utils::filesystem_identity(payload),
            expected_identity
        );
        fs::create_dir(&original).unwrap();
        fs::write(original.join("replacement.txt"), "replacement").unwrap();
        Err(crate::shared::errors::AppError::Io(
            "injected recycle failure".to_string(),
        ))
    });
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        fs::read_to_string(original.join("replacement.txt")).unwrap(),
        "replacement"
    );
    assert!(fs::read_dir(&mods).unwrap().flatten().any(|entry| entry
        .file_name()
        .to_string_lossy()
        .starts_with(".emmm-trash-cleanup-")
        && entry.path().join("owned.txt").exists()));
}

#[cfg(windows)]
#[test]
fn startup_cleanup_refuses_a_junction_even_when_its_target_identity_matches() {
    use super::service::finalize_recorded_quarantines_with;
    let (_root, mods) = setup_mods();
    let source = mods.join("Mod");
    fs::create_dir(&source).unwrap();
    let prepared = prepare_trash_move(&source).unwrap();
    prepared.execute(&WatcherState::new()).unwrap();
    let retained = mods.join("retained");
    fs::rename(prepared.quarantine(), &retained).unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(prepared.quarantine())
        .arg(&retained)
        .status()
        .unwrap();
    assert!(status.success());
    let record = RecordedQuarantine {
        path: prepared.quarantine().to_path_buf(),
        source_path: Some(source),
        game_root: mods.clone(),
        expected_root_identity: crate::platform::fs::file_utils::filesystem_identity(&mods),
        expected_identity: Some(prepared.expected_identity().to_string()),
    };
    assert_eq!(
        crate::platform::fs::file_utils::filesystem_identity(&record.path),
        record.expected_identity
    );
    let warnings = finalize_recorded_quarantines_with([record], &|_| {
        panic!("junction must not reach recycle")
    });
    assert_eq!(warnings.len(), 1);
    fs::remove_dir(prepared.quarantine()).unwrap();
    assert!(retained.exists());
}
