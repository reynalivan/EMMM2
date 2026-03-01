use super::*;
use crate::services::scanner::watcher::WatcherState;
use std::fs;
use tempfile::TempDir;

// We will test `import_mods_from_paths` directly.
// Note: Actual sevenz-rust extraction needs real archives. We can test failures on bad paths or non-archives,
// and we can simulate a fake archive by creating a dummy zip file (if possible) or just test failure branches.

#[tokio::test]
async fn test_import_mods_target_not_exists() {
    let tmp = TempDir::new().unwrap();
    let state = WatcherState::new();

    // Passing a non-existent target dir
    let res = ingest_dropped_folders_inner(
        &state,
        vec!["/dummy/path".to_string()],
        tmp.path().join("missing").to_string_lossy().to_string(),
    )
    .await;

    assert!(res.is_err());
    assert!(res.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_ingest_dropped_folders_success() {
    let tmp = TempDir::new().unwrap();
    let source_dir = tmp.path().join("source_mod");
    let target_dir = tmp.path().join("target");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&target_dir).unwrap();

    let state = WatcherState::new();

    let res = ingest_dropped_folders_inner(
        &state,
        vec![source_dir.to_string_lossy().to_string()],
        target_dir.to_string_lossy().to_string(),
    )
    .await
    .unwrap();

    assert_eq!(res.moved.len(), 1);
    assert_eq!(res.moved[0], "source_mod");
    assert!(target_dir.join("source_mod").exists());
}

// Since import_mods_from_paths requires AppHandle & State which are hard to mock in isolated cargo tests
// without a full Tauri builder, we rely on the inner functions or test what we can.
// EMMM2 codebase limits full mocked Tauri context in these basic tests.
