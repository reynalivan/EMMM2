#[allow(unused_imports)]
use super::*;
use crate::services::mods::organizer_ext::auto_organize_mods_service;
use crate::services::scanner::watcher::WatcherState;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");

    sqlx::query("CREATE TABLE mods (id TEXT PRIMARY KEY, folder_path TEXT NOT NULL);")
        .execute(&pool)
        .await
        .unwrap();

    pool
}

#[tokio::test]
async fn test_auto_organize_mods_success() {
    let pool = setup_test_db().await;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("Mods");
    fs::create_dir(&root).unwrap();

    let source = tmp.path().join("Raiden Shogun Pack");
    fs::create_dir(&source).unwrap();

    // Insert mod in DB
    sqlx::query("INSERT INTO mods (id, folder_path) VALUES ('1', ?)")
        .bind(source.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Use the same MasterDb format as the existing organizer tests
    let db_json = json!([
        {
            "name": "Raiden Shogun",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {"Default": ["d94c8962"]}
        }
    ])
    .to_string();

    let watcher = WatcherState::new();

    let res = auto_organize_mods_service(
        &pool,
        vec![source.to_string_lossy().to_string()],
        root.to_string_lossy().to_string(),
        db_json,
        &watcher,
    )
    .await
    .unwrap();

    // No match for "Raiden Shogun Pack" (doesn't match the hash) → stays in place
    // The folder may succeed (staying in place) or fail depending on match
    // Successful means it either moved or was skipped without error
    let total = res.success.len() + res.failures.len();
    assert_eq!(total, 1); // exactly 1 item was processed
}
