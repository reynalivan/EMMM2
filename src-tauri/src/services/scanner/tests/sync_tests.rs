use super::*;
use crate::services::scanner::deep_matcher;
use crate::services::scanner::sync::helpers::{
    canonical_entry_key, resolve_or_create_object_target_for_match, ResolveObjectTargetInput,
};
use serde_json::json;

use sqlx::Row;
use sqlx::SqlitePool;
use std::fs;
use tempfile::TempDir;

async fn test_pool() -> SqlitePool {
    crate::test_utils::init_test_db().await.pool
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
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideSunset]\nhash=12345678",
    )
    .unwrap();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/"),
        },
    )
    .await
    .unwrap();

    let db = needs_review_db();
    let items = scan_preview(&pool, "g1", temp_dir.path(), &db, None, None, None)
        .await
        .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].match_level, "NeedsReview");
    assert!(items[0].matched_entry_key.is_none());
    assert!(items[0].matched_alias_name.is_none());
}

// Covers: TC-2.3-Review-06 (Commit auto-links unmatched item to "Other" object)
#[tokio::test]
async fn test_commit_scan_results_non_auto_links_to_other() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("Sunset Pack");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("mod.ini"), "").unwrap();

    let items = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "Sunset Pack".to_string(),
        is_disabled: false,
        matched_entry_key: None,
        matched_alias_name: None,
        matched_confidence: None,
        matched_reason: None,
        object_type: None,
        thumbnail_path: None,
        tags_json: None,
        metadata_json: None,
        hash_db_json: None,
        custom_skins_json: None,
        db_thumbnail: None,
        skip: false,
        move_from_temp: false,
    }];

    let _result = commit_scan_results(CommitScanRequest {
        pool: &pool,
        game_id: "g1",
        game_name: "Game One",
        game_type: "gimi",
        mods_path: &temp_dir.path().to_string_lossy(),
        items,
        resource_dir: None,
        safe_mode_keywords: &[],
        preserve_existing_mappings: false,
    })
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
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideHook]\nhash=12345678",
    )
    .unwrap();

    // First commit: unmatched → obj_name = "hook" (Other, no thumbnail)
    let items_unmatched = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "hook".to_string(),
        is_disabled: false,
        matched_entry_key: None,
        matched_alias_name: None,
        matched_confidence: None,
        matched_reason: None,
        object_type: None,
        thumbnail_path: None,
        tags_json: None,
        metadata_json: None,
        hash_db_json: None,
        custom_skins_json: None,
        db_thumbnail: None,
        skip: false,
        move_from_temp: false,
    }];

    let _r1 = commit_scan_results(CommitScanRequest {
        pool: &pool,
        game_id: "g1",
        game_name: "Game",
        game_type: "srmi",
        mods_path: &temp_dir.path().to_string_lossy(),
        items: items_unmatched,
        resource_dir: None,
        safe_mode_keywords: &[],
        preserve_existing_mappings: false,
    })
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

    // Second commit enriches the same physical object with canonical relation data.
    let items_matched = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "hook".to_string(),
        is_disabled: false,
        matched_entry_key: Some(canonical_entry_key("Hook")),
        matched_alias_name: Some("Hook".to_string()),
        matched_confidence: Some(0.98),
        matched_reason: Some("ExactAlias".to_string()),
        object_type: Some("Character".to_string()),
        thumbnail_path: Some("thumbnails/hook.png".to_string()),
        tags_json: Some(r#"["fire"]"#.to_string()),
        metadata_json: Some(r#"{"rarity":"4-Star"}"#.to_string()),
        hash_db_json: None,
        custom_skins_json: None,
        db_thumbnail: None,
        skip: false,
        move_from_temp: false,
    }];

    let _r2 = commit_scan_results(CommitScanRequest {
        pool: &pool,
        game_id: "g1",
        game_name: "Game",
        game_type: "srmi",
        mods_path: &temp_dir.path().to_string_lossy(),
        items: items_matched,
        resource_dir: None,
        safe_mode_keywords: &[],
        preserve_existing_mappings: false,
    })
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

    // Verify: the surviving object keeps its physical identity and stores canonical relation.
    let obj_row = sqlx::query(
        "SELECT name, object_type, thumbnail_path, matched_entry_key, matched_alias_name
         FROM objects WHERE game_id = ?",
    )
    .bind("g1")
    .fetch_one(&pool)
    .await
    .unwrap();
    let name: String = obj_row.try_get("name").unwrap();
    let obj_type: String = obj_row.try_get("object_type").unwrap();
    let thumb: Option<String> = obj_row.try_get("thumbnail_path").unwrap();
    let matched_entry_key: Option<String> = obj_row.try_get("matched_entry_key").unwrap();
    let matched_alias_name: Option<String> = obj_row.try_get("matched_alias_name").unwrap();
    assert_eq!(name, "hook", "Physical object name must remain unchanged");
    assert_eq!(
        obj_type, "Character",
        "Type should still be enriched from canonical match"
    );
    assert!(thumb.is_some(), "Thumbnail should be set from matched data");
    assert_eq!(matched_entry_key.as_deref(), Some("hook"));
    assert_eq!(matched_alias_name.as_deref(), Some("Hook"));
}

// Covers: TC-27-001 & TC-27-004 (Atomic Upsert and Auto-Create Object link)
#[tokio::test]
async fn test_commit_creates_new_mods_and_objects_safely() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("Kazuha_New");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideKazuha]\nhash=12345678",
    )
    .unwrap();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/"),
        },
    )
    .await
    .unwrap();

    let items = vec![ConfirmedScanItem {
        folder_path: mod_dir.to_string_lossy().to_string(),
        display_name: "Kazuha New".to_string(),
        is_disabled: false,
        matched_entry_key: Some(canonical_entry_key("Kazuha")),
        matched_alias_name: Some("Kazuha".to_string()),
        matched_confidence: Some(0.91),
        matched_reason: Some("Alias".to_string()),
        object_type: Some("Character".to_string()),
        thumbnail_path: None,
        tags_json: Some("[]".to_string()),
        metadata_json: Some("{}".to_string()),
        hash_db_json: None,
        custom_skins_json: None,
        db_thumbnail: None,
        skip: false,
        move_from_temp: false,
    }];

    let result = commit_scan_results(CommitScanRequest {
        pool: &pool,
        game_id: "g1",
        game_name: "Game",
        game_type: "gimi",
        mods_path: &temp_dir.path().to_string_lossy(),
        items,
        resource_dir: None,
        safe_mode_keywords: &[],
        preserve_existing_mappings: false,
    })
    .await
    .unwrap();

    assert_eq!(result.new_mods, 1);
    assert_eq!(result.new_objects, 1);

    // Verify DB linkage
    let object_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM objects WHERE game_id = ? AND matched_entry_key = ?",
    )
    .bind("g1")
    .bind(canonical_entry_key("Kazuha"))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(object_count, 1);
}

#[tokio::test]
async fn test_resolve_or_create_object_target_for_match_creates_and_reuses_physical_shell() {
    let pool = test_pool().await;
    let temp_dir = TempDir::new().unwrap();
    let mods_path = temp_dir.path().to_string_lossy().to_string();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some(&mods_path),
        },
    )
    .await
    .unwrap();

    let matched_entry_key = canonical_entry_key("Navia");
    let mut tx = pool.begin().await.unwrap();
    let mut new_objects_count = 0_usize;

    let created = resolve_or_create_object_target_for_match(
        &mut *tx,
        ResolveObjectTargetInput {
            game_id: "g1",
            mods_path: &mods_path,
            physical_name_hint: "Navia Import Pack",
            matched_entry_key: Some(&matched_entry_key),
            object_type: "Character",
            db_thumbnail: None,
            db_tags_json: "[]",
            db_metadata_json: "{}",
            db_hash_db_json: None,
            db_custom_skins_json: None,
        },
        &mut new_objects_count,
    )
    .await
    .unwrap()
    .expect("expected physical object shell to be created");

    crate::repo::object_repo::apply_canonical_match(
        &mut *tx,
        &created.object_id,
        Some(&matched_entry_key),
        Some("Navia"),
        Some(0.92),
        Some("AutoMatched"),
        Some("deepmatch_scanner"),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(created.folder_path, "Navia Import Pack");
    assert_eq!(new_objects_count, 1);

    let mut tx = pool.begin().await.unwrap();
    let mut second_new_objects_count = 0_usize;
    let reused = resolve_or_create_object_target_for_match(
        &mut *tx,
        ResolveObjectTargetInput {
            game_id: "g1",
            mods_path: &mods_path,
            physical_name_hint: "Another Physical Name",
            matched_entry_key: Some(&matched_entry_key),
            object_type: "Character",
            db_thumbnail: None,
            db_tags_json: "[]",
            db_metadata_json: "{}",
            db_hash_db_json: None,
            db_custom_skins_json: None,
        },
        &mut second_new_objects_count,
    )
    .await
    .unwrap()
    .expect("expected existing canonical object shell to be reused");
    tx.rollback().await.unwrap();

    assert_eq!(reused.object_id, created.object_id);
    assert_eq!(reused.folder_path, created.folder_path);
    assert_eq!(second_new_objects_count, 0);

    let object_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM objects WHERE game_id = ? AND matched_entry_key = ?",
    )
    .bind("g1")
    .bind(&matched_entry_key)
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
    fs::write(
        mod1_dir.join("mod.ini"),
        "[TextureOverrideValid]\nhash=12345678",
    )
    .unwrap();

    // Invalid item (will fail because source doesn't exist but move_from_temp = true)
    let mod2_dir = temp_dir.path().join("Missing_Temp_Mod");

    let items = vec![
        ConfirmedScanItem {
            folder_path: mod1_dir.to_string_lossy().to_string(),
            display_name: "Valid".to_string(),
            is_disabled: false,
            matched_entry_key: Some(canonical_entry_key("Valid_Obj")),
            matched_alias_name: Some("Valid_Obj".to_string()),
            matched_confidence: Some(0.75),
            matched_reason: Some("Test".to_string()),
            object_type: Some("Character".to_string()),
            thumbnail_path: None,
            tags_json: None,
            metadata_json: None,
            hash_db_json: None,
            custom_skins_json: None,
            db_thumbnail: None,
            skip: false,
            move_from_temp: false,
        },
        ConfirmedScanItem {
            folder_path: mod2_dir.to_string_lossy().to_string(),
            display_name: "Invalid Move".to_string(),
            is_disabled: false,
            matched_entry_key: Some(canonical_entry_key("Crash_Obj")),
            matched_alias_name: Some("Crash_Obj".to_string()),
            matched_confidence: Some(0.75),
            matched_reason: Some("Test".to_string()),
            object_type: Some("Character".to_string()),
            thumbnail_path: None,
            tags_json: None,
            metadata_json: None,
            hash_db_json: None,
            custom_skins_json: None,
            db_thumbnail: None,
            skip: false,
            move_from_temp: true, // Will fail because source dir doesn't exist
        },
    ];

    let result = commit_scan_results(CommitScanRequest {
        pool: &pool,
        game_id: "g1",
        game_name: "Game",
        game_type: "gimi",
        mods_path: &temp_dir.path().to_string_lossy(),
        items,
        resource_dir: None,
        safe_mode_keywords: &[],
        preserve_existing_mappings: false,
    })
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

    // Insert game first to satisfy FK
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

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
    let _ = commit_scan_results(CommitScanRequest {
        pool: &pool,
        game_id: "g1",
        game_name: "Game",
        game_type: "gimi",
        mods_path: &temp_dir.path().to_string_lossy(),
        items: vec![],
        resource_dir: None,
        safe_mode_keywords: &[],
        preserve_existing_mappings: false,
    })
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
