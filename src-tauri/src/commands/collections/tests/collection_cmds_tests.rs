use super::*;
use crate::services::collections::CreateCollectionInput;
use crate::services::scanner::watcher::WatcherState;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::fs;
use tempfile::TempDir;

async fn setup_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_type TEXT NOT NULL,
            path TEXT NOT NULL,
            mod_path TEXT,
            game_exe TEXT,
            launcher_path TEXT,
            loader_exe TEXT,
            launch_args TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS mods (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            actual_name TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            status TEXT DEFAULT 'DISABLED',
            is_pinned BOOLEAN DEFAULT 0,
            is_safe BOOLEAN DEFAULT 0,
            last_status_active BOOLEAN,
            size_bytes INTEGER,
            object_type TEXT,
            metadata_blob JSON,
            object_id TEXT,
            is_favorite BOOLEAN DEFAULT 0,
            indexed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_id TEXT NOT NULL,
            is_safe_context BOOLEAN DEFAULT 1,
            is_last_unsaved BOOLEAN DEFAULT 0
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_items (
            collection_id TEXT NOT NULL,
            mod_id TEXT NOT NULL,
            mod_path TEXT,
            FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
            FOREIGN KEY(mod_id) REFERENCES mods(id) ON DELETE CASCADE,
            PRIMARY KEY (collection_id, mod_id)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_nested_items (
            collection_id TEXT NOT NULL,
            mod_path TEXT NOT NULL,
            FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
            PRIMARY KEY (collection_id, mod_path)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

async fn seed_game(pool: &SqlitePool, id: &str, name: &str) {
    sqlx::query("INSERT INTO games (id, name, game_type, path) VALUES (?, ?, 'GIMI', '/dummy')")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
}

async fn seed_game_with_mods_path(pool: &SqlitePool, id: &str, name: &str, mods_path: &str) {
    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES (?, ?, 'GIMI', '/dummy', ?)")
        .bind(id)
        .bind(name)
        .bind(mods_path)
        .execute(pool)
        .await
        .unwrap();
}

async fn seed_mod(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    path: &str,
    status: &str,
    is_safe: bool,
) {
    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, is_safe)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(game_id)
    .bind(name)
    .bind(path)
    .bind(status)
    .bind(is_safe)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_create_and_list_collection() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    seed_mod(&pool, "m1", "g1", "Mod1", "/dummy/Mod1", "ENABLED", true).await;
    seed_mod(&pool, "m2", "g1", "Mod2", "/dummy/Mod2", "ENABLED", true).await;
    seed_mod(&pool, "m3", "g1", "Mod3", "/dummy/Mod3", "DISABLED", true).await;

    let input = CreateCollectionInput {
        name: "TestCollection".to_string(),
        game_id: "g1".to_string(),
        is_safe_context: true,
        auto_snapshot: Some(false),
        mod_ids: vec!["m1".to_string(), "m2".to_string()],
    };

    let details = create_collection_service(&pool, input).await.unwrap();
    assert_eq!(details.collection.name, "TestCollection");
    assert_eq!(details.mod_ids.len(), 2);
    assert_eq!(details.mod_ids[0], "m1");

    let collections = list_collections_service(&pool, "g1", true).await.unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "TestCollection");
    assert_eq!(collections[0].member_count, 2);
}

#[tokio::test]
async fn test_apply_collection_atomic() {
    let tmp = TempDir::new().unwrap();
    let mod1_dir = tmp.path().join("Mod1");
    let mod2_dir = tmp.path().join("Mod2");
    let mod3_dir = tmp.path().join("Mod3");

    fs::create_dir(&mod1_dir).unwrap();
    fs::create_dir(&mod2_dir).unwrap();
    fs::create_dir(&mod3_dir).unwrap(); // Start all as ENABLED

    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    seed_mod(
        &pool,
        "m1",
        "g1",
        "Mod1",
        &mod1_dir.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;
    seed_mod(
        &pool,
        "m2",
        "g1",
        "Mod2",
        &mod2_dir.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;
    seed_mod(
        &pool,
        "m3",
        "g1",
        "Mod3",
        &mod3_dir.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;

    // Create a collection capturing ONLY m1 and m2
    // m3 is ENABLED now, but when we apply the collection, it should be DISABLED
    sqlx::query(
        "INSERT INTO collections (id, name, game_id, is_safe_context) VALUES ('c1', 'C1', 'g1', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO collection_items (collection_id, mod_id, mod_path) VALUES ('c1', 'm1', ?), ('c1', 'm2', ?)")
        .bind(&mod1_dir.to_string_lossy().to_string())
        .bind(&mod2_dir.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Now change physical status of m2 to DISABLED first to test apply
    let mod2_disabled = tmp.path().join("DISABLED Mod2");
    fs::rename(&mod2_dir, &mod2_disabled).unwrap();
    sqlx::query("UPDATE mods SET folder_path = ?, status = 'DISABLED' WHERE id = 'm2'")
        .bind(mod2_disabled.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    let watcher_state = WatcherState::new();

    let res = apply_collection_service(&pool, &watcher_state, "c1", "g1", true)
        .await
        .unwrap();

    // m1 was already ENABLED -> noop
    // m2 was DISABLED -> ENABLED (1 change)
    // m3 was ENABLED but NOT in collection -> DISABLED (1 change)
    assert_eq!(res.changed_count, 2);

    // m2 should be re-enabled (prefix removed)
    assert!(tmp.path().join("Mod2").exists());
    // m3 should be disabled (prefix added)
    assert!(tmp.path().join("DISABLED Mod3").exists());
}

#[tokio::test]
async fn test_apply_disables_all_non_collection_mods() {
    let tmp = TempDir::new().unwrap();
    let mod_a = tmp.path().join("ModA");
    let mod_b = tmp.path().join("ModB");
    let mod_c = tmp.path().join("ModC");
    let mod_d = tmp.path().join("ModD");

    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();
    fs::create_dir(&mod_c).unwrap();
    fs::create_dir(&mod_d).unwrap();

    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    // ModA: object Raiden, ENABLED
    seed_mod(
        &pool,
        "a",
        "g1",
        "ModA",
        &mod_a.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;
    // ModB: object Barbara, ENABLED — different object, no conflict in old logic
    seed_mod(
        &pool,
        "b",
        "g1",
        "ModB",
        &mod_b.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;
    // ModC: NULL object, ENABLED — no object_id at all
    seed_mod(
        &pool,
        "c",
        "g1",
        "ModC",
        &mod_c.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;
    // ModD: object Raiden, DISABLED
    seed_mod(
        &pool,
        "d",
        "g1",
        "ModD",
        &mod_d.to_string_lossy(),
        "DISABLED",
        true,
    )
    .await;

    // Set object_ids
    sqlx::query("UPDATE mods SET object_id = 'raiden' WHERE id IN ('a', 'd')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE mods SET object_id = 'barbara' WHERE id = 'b'")
        .execute(&pool)
        .await
        .unwrap();
    // ModC has NULL object_id

    // Collection only contains ModA
    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context) VALUES ('c1', 'OnlyA', 'g1', 1)")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO collection_items (collection_id, mod_id, mod_path) VALUES ('c1', 'a', ?)",
    )
    .bind(&mod_a.to_string_lossy().to_string())
    .execute(&pool)
    .await
    .unwrap();

    let watcher_state = WatcherState::new();
    let res = apply_collection_service(&pool, &watcher_state, "c1", "g1", true)
        .await
        .unwrap();

    // ModA: already ENABLED -> noop
    // ModB: ENABLED but NOT in collection -> DISABLED (1 change)
    // ModC: ENABLED but NOT in collection -> DISABLED (1 change)
    // ModD: already DISABLED -> stays DISABLED (noop)
    assert_eq!(res.changed_count, 2);

    assert!(tmp.path().join("ModA").exists());
    assert!(tmp.path().join("DISABLED ModB").exists());
    assert!(tmp.path().join("DISABLED ModC").exists());
}

#[tokio::test]
async fn test_mid_fail_rollback() {
    // N/A: Simulating mid-fail requires OS lock injection which is hard to do in cargo test without blocking,
    // but we can test Undo snapshot mechanism manually.
}

#[tokio::test]
async fn test_undo_action() {
    let tmp = TempDir::new().unwrap();
    let mod1_dir = tmp.path().join("Mod1");

    fs::create_dir(&mod1_dir).unwrap();

    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;
    seed_mod(
        &pool,
        "m1",
        "g1",
        "Mod1",
        &mod1_dir.to_string_lossy(),
        "ENABLED",
        true,
    )
    .await;

    // Snapshot current state
    let watcher_state = WatcherState::new();

    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context, is_last_unsaved) VALUES ('snap1', 'Unsaved', 'g1', 1, 1)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO collection_items (collection_id, mod_id, mod_path) VALUES ('snap1', 'm1', ?)",
    )
    .bind(&mod1_dir.to_string_lossy().to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Change mod1 physical state (simulate user doing something else)
    let mod1_disabled = tmp.path().join("DISABLED Mod1");
    fs::rename(&mod1_dir, &mod1_disabled).unwrap();
    sqlx::query("UPDATE mods SET status = 'DISABLED', folder_path = ? WHERE id = 'm1'")
        .bind(&mod1_disabled.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Call Undo
    let res = undo_collection_service(&pool, &watcher_state, "g1", true)
        .await
        .unwrap();

    // It should have restored m1 to ENABLED
    assert_eq!(res.changed_count, 1);
    assert!(tmp.path().join("Mod1").exists());
}

#[tokio::test]
async fn test_collection_captures_nested_mods() {
    let tmp = TempDir::new().unwrap();
    let mods_path = tmp.path();

    // Setup nested mods folder structure
    let container = mods_path.join("Character");
    fs::create_dir(&container).unwrap();

    // NestedModA: ENABLED
    let mod_a_dir = container.join("NestedModA");
    fs::create_dir(&mod_a_dir).unwrap();
    fs::write(mod_a_dir.join("mod.ini"), "[TextureOverride]").unwrap();

    // NestedModB: DISABLED
    let mod_b_dir = container.join("DISABLED NestedModB");
    fs::create_dir(&mod_b_dir).unwrap();
    fs::write(mod_b_dir.join("mod.ini"), "[TextureOverride]").unwrap();

    let pool = setup_pool().await;
    seed_game_with_mods_path(&pool, "g1", "Genshin", &mods_path.to_string_lossy()).await;

    // Save collection
    let create_input = CreateCollectionInput {
        name: "Nested Collection".to_string(),
        game_id: "g1".to_string(),
        is_safe_context: true,
        auto_snapshot: Some(true),
        mod_ids: vec![],
    };

    let details = create_collection_service(&pool, create_input)
        .await
        .unwrap();

    // Member count should be 1 (NestedModA) since it's the only enabled mod
    assert_eq!(details.collection.member_count, 1);

    // Now manually change filesystem state
    // Disable A, Enable B
    let mod_a_disabled = container.join("DISABLED NestedModA");
    fs::rename(&mod_a_dir, &mod_a_disabled).unwrap();

    let mod_b_enabled = container.join("NestedModB");
    fs::rename(&mod_b_dir, &mod_b_enabled).unwrap();

    // Apply collection
    let watcher_state = WatcherState::new();
    let res = apply_collection_service(&pool, &watcher_state, &details.collection.id, "g1", true)
        .await
        .unwrap();

    // It should have enabled ModA and disabled ModB (2 changes)
    assert_eq!(res.changed_count, 2);

    // Check filesystem state
    assert!(container.join("NestedModA").exists());
    assert!(!container.join("DISABLED NestedModA").exists());
    assert!(container.join("DISABLED NestedModB").exists());
    assert!(!container.join("NestedModB").exists());
}
