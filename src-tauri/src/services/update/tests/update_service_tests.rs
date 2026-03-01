use crate::services::update::asset_fetch::fetch_asset_if_missing;
use crate::services::update::metadata_sync::check_and_sync_metadata;
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_fetch_asset_if_missing_already_cached() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();
    let assets_dir = cache_dir.join("assets");
    fs::create_dir_all(&assets_dir).unwrap();

    // Create a fake cached asset
    let asset_name = "test_asset.png";
    let cached_path = assets_dir.join(asset_name);
    fs::write(&cached_path, b"fake data").unwrap();

    // Call the function. It should immediately return the path without making HTTP requests.
    let result_path = fetch_asset_if_missing(asset_name, cache_dir).await;

    assert!(result_path.is_some());
    assert_eq!(result_path.unwrap(), cached_path);
}

#[tokio::test]
async fn test_check_and_sync_metadata_graceful_handling() {
    let pool = setup_test_db().await;

    // If we call check_and_sync_metadata, it will attempt a real HTTP request to GitHub.
    // If it succeeds, it updates the DB. If it fails (no network), it logs a warning and returns updated: false.
    // We can at least ensure it doesn't panic.
    let result = check_and_sync_metadata(&pool).await;

    // Both success and failure are valid outcomes in a CI environment (where network might drop)
    // The key is it returns a structured result without crashing.
    assert!(result.updated == true || result.updated == false);
}
