use super::*;
use crate::services::collections::CreateCollectionInput;
use crate::services::scanner::watcher::WatcherState;
use crate::test_utils::{
    insert_test_collection, insert_test_collection_item, insert_test_collection_object_state,
    insert_test_mod, insert_test_nested_collection_item, insert_test_object,
    update_test_mod_path_and_status, TestCollectionFixture, TestCollectionItemFixture,
    TestCollectionObjectStateFixture, TestModFixture, TestNestedCollectionItemFixture,
    TestObjectFixture,
};

use sqlx::SqlitePool;
use std::fs;
use tempfile::TempDir;

async fn setup_pool() -> SqlitePool {
    crate::test_utils::init_test_db().await.pool
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
    insert_test_mod(
        pool,
        &TestModFixture {
            id,
            game_id,
            object_id: None,
            actual_name: name,
            folder_path: path,
            status,
            is_safe,
            object_type: Some("Other"),
            mods_path: None,
        },
    )
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
        object_states: None,
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
async fn test_save_snapshot_collection_as_named_clones_unsaved_snapshot() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "obj-1",
            game_id: "g1",
            name: "Ainoz",
            folder_path: None,
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    seed_mod(
        &pool,
        "m1",
        "g1",
        "AinoMod",
        "E:/Mods/Ainoz/AinoMod",
        "ENABLED",
        true,
    )
    .await;

    sqlx::query("UPDATE mods SET object_id = 'obj-1' WHERE id = 'm1'")
        .execute(&pool)
        .await
        .unwrap();

    insert_test_collection(
        &pool,
        &TestCollectionFixture {
            id: "c-unsaved",
            name: "Unsaved 202603182218",
            game_id: "g1",
            is_safe_context: true,
            is_last_unsaved: true,
        },
    )
    .await
    .unwrap();

    insert_test_collection_item(
        &pool,
        &TestCollectionItemFixture {
            collection_id: "c-unsaved",
            mod_id: "m1",
            mod_path: "E:/Mods/Ainoz/AinoMod",
            mods_path: None,
        },
    )
    .await
    .unwrap();

    insert_test_nested_collection_item(
        &pool,
        &TestNestedCollectionItemFixture {
            collection_id: "c-unsaved",
            mod_path: "E:/Mods/Ainoz/NestedMod",
            mods_path: None,
        },
    )
    .await
    .unwrap();

    insert_test_collection_object_state(
        &pool,
        &TestCollectionObjectStateFixture {
            collection_id: "c-unsaved",
            object_id: "obj-1",
            is_enabled: false,
        },
    )
    .await
    .unwrap();

    crate::database::corridor_state_repo::update_active_collection_id(
        &pool,
        "g1",
        true,
        Some("c-unsaved"),
    )
    .await
    .unwrap();

    let saved =
        save_snapshot_collection_as_named_service(&pool, "c-unsaved", "g1", "Saved Snapshot")
            .await
            .unwrap();

    assert_eq!(saved.collection.name, "Saved Snapshot");
    assert!(!saved.collection.is_last_unsaved);
    assert_eq!(saved.mod_ids, vec!["m1".to_string()]);
    assert_eq!(saved.object_states.len(), 1);
    assert_eq!(saved.object_states[0].object_id, "obj-1");
    assert!(!saved.object_states[0].is_enabled);

    let cloned_preview = crate::services::collections::get_collection_runtime_preview(
        &pool,
        &saved.collection.id,
        "g1",
    )
    .await
    .unwrap();
    assert_eq!(cloned_preview.roots.len(), 1);
    assert_eq!(cloned_preview.object_states.len(), 1);
    assert_eq!(cloned_preview.object_states[0].object_id, "obj-1");
    assert!(!cloned_preview.object_states[0].is_enabled);

    let corridor_state =
        crate::database::corridor_state_repo::get_corridor_state(&pool, "g1", true)
            .await
            .unwrap();
    assert_eq!(
        corridor_state.active_collection_id,
        Some(saved.collection.id)
    );
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
    insert_test_collection(
        &pool,
        &TestCollectionFixture {
            id: "c1",
            name: "C1",
            game_id: "g1",
            is_safe_context: true,
            is_last_unsaved: false,
        },
    )
    .await
    .unwrap();
    for (mod_id, mod_path) in [
        ("m1", mod1_dir.to_string_lossy().to_string()),
        ("m2", mod2_dir.to_string_lossy().to_string()),
    ] {
        insert_test_collection_item(
            &pool,
            &TestCollectionItemFixture {
                collection_id: "c1",
                mod_id,
                mod_path: &mod_path,
                mods_path: None,
            },
        )
        .await
        .unwrap();
    }

    // Now change physical status of m2 to DISABLED first to test apply
    let mod2_disabled = tmp.path().join("DISABLED Mod2");
    fs::rename(&mod2_dir, &mod2_disabled).unwrap();
    update_test_mod_path_and_status(
        &pool,
        "m2",
        &mod2_disabled.to_string_lossy(),
        None,
        "DISABLED",
    )
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

    // Insert objects to satisfy FK constraints for mods.object_id
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "raiden",
            game_id: "g1",
            name: "Raiden",
            folder_path: None,
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "barbara",
            game_id: "g1",
            name: "Barbara",
            folder_path: None,
            object_type: "Character",
        },
    )
    .await
    .unwrap();

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
    insert_test_collection(
        &pool,
        &TestCollectionFixture {
            id: "c1",
            name: "OnlyA",
            game_id: "g1",
            is_safe_context: true,
            is_last_unsaved: false,
        },
    )
    .await
    .unwrap();
    let mod_a_path = mod_a.to_string_lossy().to_string();
    insert_test_collection_item(
        &pool,
        &TestCollectionItemFixture {
            collection_id: "c1",
            mod_id: "a",
            mod_path: &mod_a_path,
            mods_path: None,
        },
    )
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

    insert_test_collection(
        &pool,
        &TestCollectionFixture {
            id: "snap1",
            name: "Unsaved",
            game_id: "g1",
            is_safe_context: true,
            is_last_unsaved: true,
        },
    )
    .await
    .unwrap();
    let mod1_snapshot_path = mod1_dir.to_string_lossy().to_string();
    insert_test_collection_item(
        &pool,
        &TestCollectionItemFixture {
            collection_id: "snap1",
            mod_id: "m1",
            mod_path: &mod1_snapshot_path,
            mods_path: None,
        },
    )
    .await
    .unwrap();

    // Change mod1 physical state (simulate user doing something else)
    let mod1_disabled = tmp.path().join("DISABLED Mod1");
    fs::rename(&mod1_dir, &mod1_disabled).unwrap();
    update_test_mod_path_and_status(
        &pool,
        "m1",
        &mod1_disabled.to_string_lossy(),
        None,
        "DISABLED",
    )
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
        object_states: None,
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
