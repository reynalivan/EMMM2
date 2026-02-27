use super::*;
use crate::types::errors::CommandResult;
use sqlx::SqlitePool;
use std::fs;
use tauri::State;
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

    // We expect the object to be indexed
    assert_eq!(objects.len(), 1, "Expected 1 object to be discovered");

    let obj = &objects[0];
    // The visual name should NOT contain the "DISABLED " prefix
    assert_eq!(
        obj.name, "MyFallbackMod",
        "Object name should have the prefix stripped"
    );
    // The folder path MUST contain the prefix because it reflects the physical disk
    assert_eq!(
        obj.folder_path, "DISABLED MyFallbackMod",
        "Folder path must reflect physical directory"
    );

    Ok(())
}
