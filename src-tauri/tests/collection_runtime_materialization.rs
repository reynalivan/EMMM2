use emmm2_lib::services::collections::{
    create_collection, save_snapshot_collection_as_named, snapshot_current_state,
    CreateCollectionInput,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

mod common;

async fn setup_pool() -> sqlx::SqlitePool {
    let ctx = common::init_test_db().await;
    ctx.pool
}

async fn seed_runtime_ready_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_dir: &Path,
) -> (String, String) {
    let object_id = format!("{game_id}-obj");
    let mod_id = format!("{game_id}-mod");
    let object_folder = "Raiden Shogun";
    let mod_folder = format!("{object_folder}/RaidenB");

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, ?, ?, ?)")
        .bind(game_id)
        .bind("Genshin")
        .bind("GIMI")
        .bind(mods_dir.to_string_lossy().to_string())
        .bind(mods_dir.to_string_lossy().to_string())
        .execute(pool)
        .await
        .expect("insert game");

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, object_type, folder_path, sort_order, created_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&object_id)
    .bind(game_id)
    .bind("Raiden Shogun")
    .bind("Character")
    .bind(object_folder)
    .bind(0)
    .execute(pool)
    .await
    .expect("insert object");

    let mod_dir = mods_dir.join(object_folder).join("RaidenB");
    fs::create_dir_all(&mod_dir).expect("create mod directory");
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideMain]\nhash = abc123\n",
    )
    .expect("write mod.ini");

    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_id)
         VALUES (?, ?, ?, ?, 'ENABLED', ?)",
    )
    .bind(&mod_id)
    .bind(game_id)
    .bind("RaidenB")
    .bind(&mod_folder)
    .bind(&object_id)
    .execute(pool)
    .await
    .expect("insert mod");

    common::refresh_unicode_keys(pool).await;

    (object_id, mod_id)
}

async fn count_runtime_materialization(pool: &sqlx::SqlitePool, collection_id: &str) -> (i64, i64) {
    let roots: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM collection_roots WHERE collection_id = ?")
            .bind(collection_id)
            .fetch_one(pool)
            .await
            .expect("count roots");
    let signatures: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM collection_signatures WHERE collection_id = ?")
            .bind(collection_id)
            .fetch_one(pool)
            .await
            .expect("count signatures");

    (roots, signatures)
}

#[tokio::test]
async fn save_current_collection_materializes_roots_and_signature() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let game_id = "game-save-current";
    let (_, mod_id) = seed_runtime_ready_game(&pool, game_id, tmp.path()).await;

    let created = create_collection(
        &pool,
        CreateCollectionInput {
            name: "Saved Current".to_string(),
            game_id: game_id.to_string(),
            is_safe_context: true,
            auto_snapshot: Some(true),
            mod_ids: vec![mod_id],
            object_states: None,
        },
    )
    .await
    .expect("save current collection");

    let (roots, signatures) = count_runtime_materialization(&pool, &created.collection.id).await;

    assert!(roots > 0);
    assert_eq!(signatures, 1);
}

#[tokio::test]
async fn snapshot_current_state_materializes_roots_and_signature() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let game_id = "game-snapshot-current";
    let _ = seed_runtime_ready_game(&pool, game_id, tmp.path()).await;

    let snapshot_id = snapshot_current_state(&pool, game_id, true)
        .await
        .expect("snapshot current state");

    let (roots, signatures) = count_runtime_materialization(&pool, &snapshot_id).await;

    assert!(roots > 0);
    assert_eq!(signatures, 1);
}

#[tokio::test]
async fn save_snapshot_collection_as_named_materializes_roots_and_signature() {
    let pool = setup_pool().await;
    let tmp = TempDir::new().expect("create temp dir");
    let game_id = "game-save-snapshot";
    let _ = seed_runtime_ready_game(&pool, game_id, tmp.path()).await;

    let snapshot_id = snapshot_current_state(&pool, game_id, true)
        .await
        .expect("snapshot current state");

    let saved = save_snapshot_collection_as_named(&pool, &snapshot_id, game_id, "Saved Snapshot")
        .await
        .expect("save snapshot as named");

    let (roots, signatures) = count_runtime_materialization(&pool, &saved.collection.id).await;

    assert!(roots > 0);
    assert_eq!(signatures, 1);
}
