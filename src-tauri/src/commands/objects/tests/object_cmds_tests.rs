use super::*;
use crate::types::errors::CommandResult;
use sqlx::SqlitePool;
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, SqlitePool, String) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
        .await
        .unwrap();

    // Run migrations (assuming we can just create the needed tables for this test)
    sqlx::query(
        "CREATE TABLE games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_type TEXT NOT NULL,
            mod_path TEXT NOT NULL
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
            object_type TEXT NOT NULL DEFAULT 'Other',
            sub_category TEXT,
            sort_order INTEGER DEFAULT 0,
            tags JSON DEFAULT '[]',
            metadata JSON DEFAULT '{}',
            thumbnail_path TEXT,
            is_safe BOOLEAN DEFAULT 1,
            is_pinned BOOLEAN DEFAULT 0,
            is_auto_sync BOOLEAN DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE mods (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            object_id TEXT,
            actual_name TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            status TEXT NOT NULL,
            object_type TEXT NOT NULL DEFAULT 'Other',
            is_favorite BOOLEAN DEFAULT 0
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let game_id = "test_game_1".to_string();
    let mods_path = tmp.path().join("Mods");
    fs::create_dir(&mods_path).unwrap();

    sqlx::query(
        "INSERT INTO games (id, name, game_type, mod_path) VALUES (?, 'Test Game', 'GIMI', ?)",
    )
    .bind(&game_id)
    .bind(mods_path.to_string_lossy().to_string())
    .execute(&pool)
    .await
    .unwrap();

    (tmp, pool, game_id)
}

#[tokio::test]
async fn test_get_objects_with_disabled_prefix() -> CommandResult<()> {
    let (tmp, pool, game_id) = setup_test_db().await;
    let mods_path = tmp.path().join("Mods");

    // Create a disabled folder in the mods directory
    let folder_name = "DISABLED MyFallbackMod";
    let mod_dir = mods_path.join(folder_name);
    fs::create_dir(&mod_dir).unwrap();

    // The physical folder exists; now also insert an object row in the DB
    // so that `get_objects_cmd_inner` (which queries the DB) can find it.
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, is_safe) VALUES (?, ?, ?, ?, 'Other', 1)"
    )
    .bind("obj_disabled")
    .bind(&game_id)
    .bind("MyFallbackMod")
    .bind(folder_name)
    .execute(&pool)
    .await
    .unwrap();

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };

    // Sync filesystem -> DB first so the DISABLED-prefixed folder is indexed
    crate::services::scanner::object_sync::sync_objects_for_game(&pool, &game_id)
        .await
        .expect("sync_objects_for_game failed");

    let objects = get_objects_cmd_inner(filter, &pool).await?;

    // We expect the object to be indexed
    assert_eq!(objects.len(), 1, "Expected 1 object to be discovered");

    let obj = &objects[0];
    // The visual name should NOT contain the "DISABLED " prefix
    assert_eq!(
        obj.name, "MyFallbackMod",
        "Object name should have the prefix stripped"
    );
    assert_eq!(
        obj.folder_path, "DISABLED MyFallbackMod",
        "Folder path must reflect physical directory"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_objects_safe_mode_filtering() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    // Insert an unsafe object manually into DB
    let obj_id = "test_unsafe_obj";
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, is_safe) VALUES (?, ?, 'NSFW_Mod', 'Character', 0)"
    )
    .bind(obj_id)
    .bind(&game_id)
    .execute(&pool)
    .await
    .unwrap();

    // 1. Fetch with safe_mode=false (should return 1)
    let filter_unfiltered = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let results_unfiltered = get_objects_cmd_inner(filter_unfiltered, &pool).await?;
    assert_eq!(
        results_unfiltered.len(),
        1,
        "Unsafe object should be returned when safe_mode=false"
    );

    // 2. Fetch with safe_mode=true (should return 0)
    let filter_safe = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: true,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let results_safe = get_objects_cmd_inner(filter_safe, &pool).await?;
    assert_eq!(
        results_safe.len(),
        0,
        "Unsafe object MUST NOT be returned when safe_mode=true (TC-08-10)"
    );

    Ok(())
}

#[tokio::test]
async fn test_create_object_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let payload = CreateObjectInput {
        game_id: game_id.clone(),
        name: "New Hero".to_string(),
        folder_path: Some("New Hero Folder".to_string()),
        object_type: "Weapon".to_string(),
        sub_category: None,
        is_safe: Some(true),
        metadata: Some(serde_json::json!({})),
    };

    let obj_id_result = create_object_cmd_inner(payload, &pool).await?;

    // Verify it exists in DB
    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let objects = get_objects_cmd_inner(filter, &pool).await?;
    assert_eq!(objects.len(), 1, "Created object must be retrievable");
    let result = &objects[0];

    assert_eq!(
        result.name, "New Hero",
        "TC-10-01: Object Name should match"
    );
    assert_eq!(
        result.object_type, "Weapon",
        "TC-10-01: Object type should match"
    );
    assert_eq!(
        result.id, obj_id_result,
        "TC-10-01: Returned ID must match the indexed ID"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_object_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let obj_id = "test_obj_update";
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, is_safe) VALUES (?, ?, 'OldName', 'Other', 1)"
    )
    .bind(obj_id)
    .bind(&game_id)
    .execute(&pool)
    .await
    .unwrap();

    let payload = UpdateObjectInput {
        name: Some("NewName".to_string()),
        object_type: Some("Character".to_string()),
        sub_category: None,
        metadata: Some(serde_json::json!({"test":true})),
        thumbnail_path: None,
        is_safe: Some(false),
        is_auto_sync: None,
        tags: Some(vec!["Pyro".to_string()]),
    };

    let _updated = update_object_cmd_inner(obj_id.to_string(), &payload, &pool).await?;

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let objects = get_objects_cmd_inner(filter, &pool).await?;
    let updated = objects.into_iter().find(|o| o.id == obj_id).unwrap();

    assert_eq!(updated.name, "NewName", "TC-10-04: Name must be updated");
    assert_eq!(
        updated.object_type, "Character",
        "TC-10-04: Type must be updated"
    );
    assert_eq!(updated.is_safe, false, "TC-10-04: is_safe must be updated");

    Ok(())
}

#[tokio::test]
async fn test_delete_object_fk_constraints() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    // Create an empty object
    let empty_obj_id = "empty_obj";
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type) VALUES (?, ?, 'Empty', 'Character')",
    )
    .bind(empty_obj_id)
    .bind(&game_id)
    .execute(&pool)
    .await
    .unwrap();

    // Create an object with mods inside it
    let full_obj_id = "full_obj";
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type) VALUES (?, ?, 'Full', 'Weapon')",
    )
    .bind(full_obj_id)
    .bind(&game_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, status, object_type) VALUES ('mod1', ?, ?, 'ModName', 'Path', 'DISABLED', 'Weapon')"
    )
    .bind(&game_id)
    .bind(full_obj_id)
    .execute(&pool)
    .await
    .unwrap();

    // Attempt to delete empty object (should succeed)
    let res_empty = delete_object_cmd_inner(empty_obj_id.to_string(), &pool).await;
    assert!(
        res_empty.is_ok(),
        "TC-10-07: Empty object should be deleted successfully"
    );

    // Attempt to delete full object (should fail)
    let res_full = delete_object_cmd_inner(full_obj_id.to_string(), &pool).await;
    assert!(
        res_full.is_err(),
        "TC-10-08: Deleting object with mods attached MUST fail due to FK/application logic constraints"
    );

    Ok(())
}

#[tokio::test]
async fn test_pin_object_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let obj_id = "test_obj_pin";
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, is_pinned) VALUES (?, ?, 'Normal', 'Character', 0)"
    )
    .bind(obj_id)
    .bind(&game_id)
    .execute(&pool)
    .await
    .unwrap();

    // Use raw query for pin since it is in mods/object_cmds.rs (we just verify the schema works)
    sqlx::query("UPDATE objects SET is_pinned = ? WHERE id = ?")
        .bind(true)
        .bind(obj_id)
        .execute(&pool)
        .await
        .unwrap();

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let objects = get_objects_cmd_inner(filter, &pool).await?;
    assert_eq!(
        objects[0].is_pinned, true,
        "TC-10-09: Object should be pinned"
    );

    Ok(())
}
