use crate::services::app::maintenance_service::{cleanup_old_empty_trash_entries, run_maintenance};
use crate::services::images::thumbnail_cache::ThumbnailCache;
use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_run_maintenance() {
    let temp_dir = TempDir::new().unwrap();
    let app_data_dir = temp_dir.path();
    ThumbnailCache::init(app_data_dir); // Ensure thumbnail dir exists

    let pool = setup_test_db().await;

    let msg = run_maintenance(&pool, app_data_dir).await.unwrap();
    assert!(msg.contains("Maintenance complete"));
}

#[test]
fn test_cleanup_old_empty_trash_entries() {
    let temp_dir = TempDir::new().unwrap();
    let trash_dir = temp_dir.path().join("trash");
    fs::create_dir_all(&trash_dir).unwrap();

    // 1. Create a recent empty folder -> should be skipped
    let recent_dir = trash_dir.join("recent");
    fs::create_dir(&recent_dir).unwrap();

    // 2. Create an old empty folder -> we simulate it by setting modify timestamp back!
    let old_dir = trash_dir.join("old");
    fs::create_dir(&old_dir).unwrap();
    let old_time = SystemTime::now() - Duration::from_secs(35 * 24 * 60 * 60);
    filetime::set_file_mtime(&old_dir, filetime::FileTime::from_system_time(old_time)).unwrap();

    // 3. Create an old folder with metadata.json -> should be skipped (not empty)
    let old_with_metadata = trash_dir.join("old_metadata");
    fs::create_dir(&old_with_metadata).unwrap();
    fs::write(old_with_metadata.join("metadata.json"), "{}").unwrap();
    filetime::set_file_mtime(
        &old_with_metadata,
        filetime::FileTime::from_system_time(old_time),
    )
    .unwrap();

    let removed = cleanup_old_empty_trash_entries(&trash_dir).unwrap();

    assert_eq!(removed, 1);
    assert!(!old_dir.exists());
    assert!(recent_dir.exists());
    assert!(old_with_metadata.exists());
}
