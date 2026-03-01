use super::*;
use crate::services::scanner::deep_matcher;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;
use sqlx::SqlitePool;
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
            folder_path TEXT,
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
            "hash_db": {}
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
            "hash_db": {}
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
    let items = scan_preview(&pool, "g1", temp_dir.path(), &db, None, None, None)
        .await
        .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].match_level, "NeedsReview");
    assert!(items[0].matched_object.is_none());
}

// Covers: TC-2.3-Review-06 (Commit auto-links unmatched item to "Other" object)
#[tokio::test]
async fn test_commit_scan_results_non_auto_links_to_other() {
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
        move_from_temp: false,
    }];

    let _result = commit_scan_results(
        &pool,
        "g1",
        "Game One",
        "gimi",
        &temp_dir.path().to_string_lossy(),
        items,
        None,
        &[],
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
    // Auto-linking creates an "Other" object for unconfirmed mods
    assert!(object_id.is_some());
    assert_eq!(object_type, "Other");

    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(object_count, 1);
}

// Covers: Fix for duplicate objects — case-insensitive merge in ensure_object_exists
#[tokio::test]
async fn test_ensure_object_case_insensitive_merge() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("hook");
    fs::create_dir(&mod_dir).unwrap();

    // First commit: unmatched → obj_name = "hook" (Other, no thumbnail)
    let items_unmatched = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "hook".to_string(),
        is_disabled: false,
        matched_object: None,
        object_type: None,
        thumbnail_path: None,
        tags_json: None,
        metadata_json: None,
        skip: false,
        move_from_temp: false,
    }];

    let _r1 = commit_scan_results(
        &pool,
        "g1",
        "Game",
        "srmi",
        &temp_dir.path().to_string_lossy(),
        items_unmatched,
        None,
        &[],
    )
    .await
    .unwrap();

    // Verify: object "hook" (Other) exists
    let obj_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
            .bind("g1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(obj_count_before, 1);

    // Second commit: matched → obj_name = "Hook" (Character, with thumbnail)
    let items_matched = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "hook".to_string(),
        is_disabled: false,
        matched_object: Some("Hook".to_string()),
        object_type: Some("Character".to_string()),
        thumbnail_path: Some("thumbnails/hook.png".to_string()),
        tags_json: Some(r#"["fire"]"#.to_string()),
        metadata_json: Some(r#"{"rarity":"4-Star"}"#.to_string()),
        skip: false,
        move_from_temp: false,
    }];

    let _r2 = commit_scan_results(
        &pool,
        "g1",
        "Game",
        "srmi",
        &temp_dir.path().to_string_lossy(),
        items_matched,
        None,
        &[],
    )
    .await
    .unwrap();

    // Verify: still only ONE object (merged, not duplicated)
    let obj_count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        obj_count_after, 1,
        "Should have merged, not created a duplicate"
    );

    // Verify: the surviving object has the canonical name "Hook" and type "Character"
    let obj_row =
        sqlx::query("SELECT name, object_type, thumbnail_path FROM objects WHERE game_id = ?")
            .bind("g1")
            .fetch_one(&pool)
            .await
            .unwrap();
    let name: String = obj_row.try_get("name").unwrap();
    let obj_type: String = obj_row.try_get("object_type").unwrap();
    let thumb: Option<String> = obj_row.try_get("thumbnail_path").unwrap();
    assert_eq!(name, "Hook", "Name should be upgraded to MasterDB alias");
    assert_eq!(
        obj_type, "Character",
        "Type should be upgraded to Character"
    );
    assert!(thumb.is_some(), "Thumbnail should be set from matched data");
}

// Covers: TC-27-001 & TC-27-004 (Atomic Upsert and Auto-Create Object link)
#[tokio::test]
async fn test_commit_creates_new_mods_and_objects_safely() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("Kazuha_New");
    fs::create_dir(&mod_dir).unwrap();

    let items = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "Kazuha New".to_string(),
        is_disabled: false,
        matched_object: Some("Kazuha".to_string()),
        object_type: Some("Character".to_string()),
        thumbnail_path: None,
        tags_json: Some("[]".to_string()),
        metadata_json: Some("{}".to_string()),
        skip: false,
        move_from_temp: false,
    }];

    let result = commit_scan_results(
        &pool,
        "g1",
        "Game",
        "gimi",
        &temp_dir.path().to_string_lossy(),
        items,
        None,
        &[],
    )
    .await
    .unwrap();

    assert_eq!(result.new_mods, 1);
    assert_eq!(result.new_objects, 1);

    // Verify DB linkage
    let object_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE name = 'Kazuha'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(object_count, 1);
}

// Covers: TC-27-003 (Transaction Rollback on failure)
#[tokio::test]
async fn test_commit_rolls_back_on_failure() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();

    // Valid item
    let mod1_dir = temp_dir.path().join("Valid_Mod");
    fs::create_dir(&mod1_dir).unwrap();

    // Invalid item (will fail because source doesn't exist but move_from_temp = true)
    let mod2_dir = temp_dir.path().join("Missing_Temp_Mod");

    let items = vec![
        ConfirmedScanItem {
            folder_path: mod1_dir.to_string_lossy().to_string(),
            display_name: "Valid".to_string(),
            is_disabled: false,
            matched_object: Some("Valid_Obj".to_string()),
            object_type: Some("Character".to_string()),
            thumbnail_path: None,
            tags_json: None,
            metadata_json: None,
            skip: false,
            move_from_temp: false,
        },
        ConfirmedScanItem {
            folder_path: mod2_dir.to_string_lossy().to_string(),
            display_name: "Invalid Move".to_string(),
            is_disabled: false,
            matched_object: Some("Crash_Obj".to_string()),
            object_type: Some("Character".to_string()),
            thumbnail_path: None,
            tags_json: None,
            metadata_json: None,
            skip: false,
            move_from_temp: true, // Will fail because source dir doesn't exist
        },
    ];

    let result = commit_scan_results(
        &pool,
        "g1",
        "Game",
        "gimi",
        &temp_dir.path().to_string_lossy(),
        items,
        None,
        &[],
    )
    .await;

    // Should return Err due to missing temp folder rename
    assert!(result.is_err());

    // VERIFY ROLLBACK: Neither Valid_Mod nor Crash_Obj should exist in the DB!
    let mods_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = 'g1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(mods_count, 0, "Transaction did not roll back properly!");
}

// Covers: TC-27-005 (Repair orphaned DB rows / garbage collection)
#[tokio::test]
async fn test_commit_garbage_collects_ghost_objects() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();

    // Insert an object manually that has NO associated mods
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type)
         VALUES ('ghost_id', 'g1', 'GhostObject', 'GhostPath', 'Character')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let obj_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'ghost_id'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(obj_count_before, 1, "Ghost object must exist initially");

    // Run an empty commit pass (triggering the GC hook at the end of commit_scan_results)
    let _ = commit_scan_results(
        &pool,
        "g1",
        "Game",
        "gimi",
        &temp_dir.path().to_string_lossy(),
        vec![],
        None,
        &[],
    )
    .await
    .unwrap();

    // Verify GC cleaned it up
    let obj_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'ghost_id'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        obj_count_after, 0,
        "Garbage Collector failed to clean orphan object"
    );
}
