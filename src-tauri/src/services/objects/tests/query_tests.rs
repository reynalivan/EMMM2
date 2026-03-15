use crate::services::objects::query::{
    gc_lost_objects, get_category_counts_service, get_object_by_id_service,
};
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_get_object_by_id_service() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g_get_obj', 'Genshin', 'type', '/game_get_obj')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type)
         VALUES ('o1', 'g_get_obj', 'MyObj', 'my_folder', 'Character')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let obj = get_object_by_id_service(&pool, "o1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(obj.id, "o1");
    assert_eq!(obj.name, "MyObj");
    assert_eq!(obj.object_type, "Character");

    let missing = get_object_by_id_service(&pool, "o2").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_get_category_counts_service() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g_cat_counts', 'StarRail', 'type', '/game_cat_counts')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 2 Characters safe, 1 Character unsafe, 1 Weapon safe
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type) VALUES
         ('o1', 'g_cat_counts', 'C1', 'c1', 'Character'),
         ('o2', 'g_cat_counts', 'C2', 'c2', 'Character'),
         ('o3', 'g_cat_counts', 'C3', 'c3', 'Character'),
         ('o4', 'g_cat_counts', 'W1', 'w1', 'Weapon')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Phase 1 fix: safe_mode no longer filters categories — always returns ALL counts.
    // The _safe_mode param is kept for API compatibility but ignored.
    let safe_counts = get_category_counts_service(&pool, "g_cat_counts", true)
        .await
        .unwrap();
    assert_eq!(safe_counts.len(), 2);
    let char_count = safe_counts
        .iter()
        .find(|c| c.object_type == "Character")
        .unwrap();
    // Now returns ALL characters (3), not just safe ones
    assert_eq!(char_count.count, 3);

    let all_counts = get_category_counts_service(&pool, "g_cat_counts", false)
        .await
        .unwrap();
    let char_count_all = all_counts
        .iter()
        .find(|c| c.object_type == "Character")
        .unwrap();
    assert_eq!(char_count_all.count, 3);
}

#[tokio::test]
async fn test_gc_lost_objects_removes_missing() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_path = temp_dir.path().join("mods_dir");
    fs::create_dir_all(&mod_path).unwrap();

    // Create a physical folder
    fs::create_dir(mod_path.join("clean_folder")).unwrap();

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES ('g_gc_lost', 'ZZZ', 'type', '/game_gc_lost', ?)")
        .bind(mod_path.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // Insert object with physical folder
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type)
         VALUES ('o2', 'g_gc_lost', 'Obj2', 'clean_folder', 'Character')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // A lost object (no physical folder)
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type)
         VALUES ('o3', 'g_gc_lost', 'MissingObj', 'deleted_folder', 'Character')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let lost = gc_lost_objects(&pool, "g_gc_lost").await.unwrap();

    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0], "MissingObj");

    let remaining: Vec<String> =
        sqlx::query_scalar("SELECT id FROM objects WHERE game_id = 'g_gc_lost'")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], "o2");
}
