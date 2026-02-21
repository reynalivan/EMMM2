use super::*;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;
use std::fs;
use tempfile::TempDir;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_type TEXT NOT NULL,
            path TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE objects (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            name TEXT NOT NULL,
            object_type TEXT NOT NULL,
            thumbnail_path TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            metadata TEXT NOT NULL DEFAULT '{}'
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE mods (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            actual_name TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            status TEXT NOT NULL,
            object_type TEXT NOT NULL,
            object_id TEXT,
            is_favorite INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

fn needs_review_db() -> deep_matcher::MasterDb {
    let db_json = json!([
        {
            "name": "Amber",
            "tags": ["sunset"],
            "object_type": "Character",
            "custom_skins": [
                {
                    "name": "Sunset",
                    "aliases": ["Sunset"],
                    "thumbnail_skin_path": null,
                    "rarity": null
                }
            ],
            "thumbnail_path": null,
            "metadata": null,
            "hashes": []
        },
        {
            "name": "Lisa",
            "tags": ["sunset"],
            "object_type": "Character",
            "custom_skins": [
                {
                    "name": "Sunset",
                    "aliases": ["Sunset"],
                    "thumbnail_skin_path": null,
                    "rarity": null
                }
            ],
            "thumbnail_path": null,
            "metadata": null,
            "hashes": []
        }
    ])
    .to_string();

    deep_matcher::MasterDb::from_json(&db_json).unwrap()
}

fn no_match_db() -> deep_matcher::MasterDb {
    let db_json = json!([
        {
            "name": "Raiden Shogun",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hashes": []
        }
    ])
    .to_string();

    deep_matcher::MasterDb::from_json(&db_json).unwrap()
}

fn auto_match_db() -> deep_matcher::MasterDb {
    let db_json = json!([
        {
            "name": "Raiden Shogun",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hashes": ["d94c8962"]
        }
    ])
    .to_string();

    deep_matcher::MasterDb::from_json(&db_json).unwrap()
}

// Covers: TC-2.3-Review-03 (Sync preview keeps NeedsReview pending)
#[tokio::test]
async fn test_scan_preview_needs_review_has_no_auto_assignment() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("Sunset Pack");
    fs::create_dir(&mod_dir).unwrap();

    let db = needs_review_db();
    let items = scan_preview(&pool, "g1", temp_dir.path(), &db, None, None)
        .await
        .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].match_level, "NeedsReview");
    assert!(items[0].matched_object.is_none());
}

// Covers: TC-2.3-Review-04 (NoMatch does not auto-assign object during sync)
#[tokio::test]
async fn test_sync_with_db_no_match_keeps_mod_pending() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("completely unknown package");
    fs::create_dir(&mod_dir).unwrap();

    let db = no_match_db();
    let _result = sync_with_db(
        &pool,
        "g1",
        "Game One",
        "gimi",
        temp_dir.path(),
        &db,
        None,
        None,
    )
    .await
    .unwrap();

    let mod_row = sqlx::query("SELECT object_id FROM mods WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let object_id: Option<String> = mod_row.try_get("object_id").unwrap();
    assert!(object_id.is_none());

    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(object_count, 0);
}

// Covers: TC-2.3-Review-05 (AutoMatched remains actionable in sync)
#[tokio::test]
async fn test_sync_with_db_auto_match_assigns_object() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("Mystery Mod");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(
        mod_dir.join("config.ini"),
        "[TextureOverrideBody]\nhash = d94c8962\n",
    )
    .unwrap();

    let db = auto_match_db();
    let _result = sync_with_db(
        &pool,
        "g1",
        "Game One",
        "gimi",
        temp_dir.path(),
        &db,
        None,
        None,
    )
    .await
    .unwrap();

    let mod_row = sqlx::query("SELECT object_id FROM mods WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let object_id: Option<String> = mod_row.try_get("object_id").unwrap();
    assert!(object_id.is_some());
}

// Covers: TC-2.3-Review-06 (Commit keeps non-auto item pending)
#[tokio::test]
async fn test_commit_scan_results_non_auto_does_not_assign_object() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("Sunset Pack");
    fs::create_dir(&mod_dir).unwrap();

    let items = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "Sunset Pack".to_string(),
        is_disabled: false,
        matched_object: None,
        object_type: None,
        thumbnail_path: None,
        tags_json: None,
        metadata_json: None,
        skip: false,
    }];

    let _result = commit_scan_results(
        &pool,
        "g1",
        "Game One",
        "gimi",
        &temp_dir.path().to_string_lossy(),
        items,
        None,
    )
    .await
    .unwrap();

    let mod_row = sqlx::query("SELECT object_id, object_type FROM mods WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let object_id: Option<String> = mod_row.try_get("object_id").unwrap();
    let object_type: String = mod_row.try_get("object_type").unwrap();
    assert!(object_id.is_none());
    assert_eq!(object_type, "Other");

    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(object_count, 0);
}
