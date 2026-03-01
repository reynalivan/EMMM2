use emmm2_lib::services::collections::{
    apply_collection, create_collection, list_collections, CreateCollectionInput,
};
use emmm2_lib::services::scanner::watcher::WatcherState;
use std::fs;
mod common;
use tempfile::TempDir;

async fn setup_pool() -> sqlx::SqlitePool {
    let ctx = common::init_test_db().await;
    ctx.pool
}

async fn seed_game_and_mods(
    pool: &sqlx::SqlitePool,
    mods_dir: &str,
) -> (String, String, String, String, String) {
    let game_id = "game-gimi".to_string();
    let mod_a_id = "mod-a".to_string();
    let mod_b_id = "mod-b".to_string();
    let object_id = "obj-raiden".to_string();

    sqlx::query("INSERT INTO games (id, name, game_type, path) VALUES (?, ?, ?, ?)")
        .bind(&game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(mods_dir)
        .execute(pool)
        .await
        .expect("insert game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, is_safe, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind("Raiden Shogun")
    .bind("Character")
    .bind(1) // is_safe true
    .bind(0)
    .execute(pool)
    .await
    .expect("insert object");

    let mod_a_path = format!("{mods_dir}/DISABLED RaidenA");
    let mod_b_path = format!("{mods_dir}/RaidenB");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_a_id)
    .bind(&game_id)
    .bind("Raiden A")
    .bind(&mod_a_path)
    .bind("DISABLED")
    .bind(&object_id)
    .execute(pool)
    .await
    .expect("insert mod a");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&mod_b_id)
    .bind(&game_id)
    .bind("Raiden B")
    .bind(&mod_b_path)
    .bind("ENABLED")
    .bind(&object_id)
    .execute(pool)
    .await
    .expect("insert mod b");

    (game_id, mod_a_id, mod_b_id, mod_a_path, mod_b_path)
}

#[tokio::test]
async fn collections_create_and_list() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, mod_a_id, _, _, _) = seed_game_and_mods(&pool, &mods_dir).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Abyss Team".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_a_id],
            auto_snapshot: None,
        },
    )
    .await
    .expect("create collection");

    let listed = list_collections(&pool, &game_id, true)
        .await
        .expect("list collections");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.collection.id);
    assert_eq!(listed[0].name, "Abyss Team");
}

#[tokio::test]
async fn collections_apply_then_undo_restores_state() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let mods_dir = tmp.path().to_string_lossy().to_string();
    let (game_id, mod_a_id, mod_b_id, mod_a_path, mod_b_path) =
        seed_game_and_mods(&pool, &mods_dir).await;

    fs::create_dir_all(&mod_a_path).expect("create disabled mod a folder");
    fs::create_dir_all(&mod_b_path).expect("create enabled mod b folder");

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Abyss Team".to_string(),
            game_id: game_id.clone(),
            is_safe_context: true,
            mod_ids: vec![mod_a_id.clone()],
            auto_snapshot: None,
        },
    )
    .await
    .expect("create collection");

    let watcher_state = WatcherState::new();

    let applied = apply_collection(
        &pool,
        &watcher_state,
        &created.collection.id,
        &game_id,
        false,
    )
    .await
    .expect("apply collection");

    assert_eq!(applied.changed_count, 2);

    let mod_a_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_a_id)
        .fetch_one(&pool)
        .await
        .expect("mod a status after apply");
    let mod_b_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_b_id)
        .fetch_one(&pool)
        .await
        .expect("mod b status after apply");

    assert_eq!(mod_a_status, "ENABLED");
    assert_eq!(mod_b_status, "DISABLED");

    let snapshot_id: String =
        sqlx::query_scalar("SELECT id FROM collections WHERE game_id = ? AND is_last_unsaved = 1")
            .bind(&game_id)
            .fetch_one(&pool)
            .await
            .expect("snapshot collection");

    let undo = apply_collection(&pool, &watcher_state, &snapshot_id, &game_id, false)
        .await
        .expect("undo apply via snapshot");

    assert_eq!(undo.changed_count, 2);

    let mod_a_status_after: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_a_id)
        .fetch_one(&pool)
        .await
        .expect("mod a status after undo");
    let mod_b_status_after: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
        .bind(&mod_b_id)
        .fetch_one(&pool)
        .await
        .expect("mod b status after undo");

    assert_eq!(mod_a_status_after, "DISABLED");
    assert_eq!(mod_b_status_after, "ENABLED");
}
