//! Collection runtime status tests.

use super::{get_collection_runtime_descriptor, get_collection_runtime_state};
use crate::domain::collection::{CreateCollectionInput, CreateCollectionMode};
use crate::domain::models::{GameType, ItemStatus};
use crate::domain::runtime_state::{LastChangesSource, RuntimeStatus};
use crate::repo::collection;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};

use crate::services::collection::create_collection;

#[tokio::test]
async fn runtime_descriptor_returns_only_compact_global_status_data() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mod_dir = mods_root.path().join("AINOZ").join("Blue");
    std::fs::create_dir_all(&mod_dir).expect("create mod folder");
    std::fs::write(mod_dir.join("mod.ini"), "[TextureOverrideTest]\n").expect("write mod file");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-descriptor",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert game");
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-descriptor",
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-1",
            game_id: "game-descriptor",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert mod");
    sqlx::query("UPDATE mods SET safety_source = 'manual' WHERE id = 'mod-1'")
        .execute(&ctx.pool)
        .await
        .expect("classify mod safety");

    let active = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-descriptor".to_string(),
            name: "Baseline".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save baseline");

    let descriptor = get_collection_runtime_descriptor(&ctx.pool, "game-descriptor")
        .await
        .expect("load runtime descriptor");

    assert_eq!(descriptor.game_id, "game-descriptor");
    assert_eq!(
        descriptor.active_collection_id.as_deref(),
        Some(active.id.as_str())
    );
    assert_eq!(
        descriptor.active_collection_name.as_deref(),
        Some("Baseline")
    );
    assert_eq!(descriptor.runtime_status, RuntimeStatus::Clean);
    assert_eq!(descriptor.missing_count, 0);
    assert_eq!(descriptor.counts.active_mod_count, 1);
    assert_eq!(descriptor.counts.object_count, 1);
    assert_eq!(descriptor.counts.enabled_object_count, 1);
    assert!(!descriptor.safety.is_safe);
    assert!(descriptor.safety.is_safety_classified);
    assert!(descriptor.last_changes.is_none());

    let payload = serde_json::to_value(&descriptor).expect("serialize descriptor");
    assert!(payload.get("current_mods").is_none());
    assert!(payload.get("current_objects").is_none());
    assert!(payload.get("current_tree_nodes").is_none());
    assert!(payload.get("projected_state").is_none());
    assert!(payload.get("is_dirty").is_none());
}

#[tokio::test]
async fn runtime_state_reads_the_full_runtime() {
    let ctx = init_test_db().await;

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-runtime",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert game");
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-runtime",
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "safe-mod",
            game_id: "game-runtime",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert safe mod");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "unsafe-mod",
            game_id: "game-runtime",
            object_id: Some("object-1"),
            actual_name: "Red",
            folder_path: "AINOZ/Red",
            status: ItemStatus::Enabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert unsafe mod");

    let snapshot = get_collection_runtime_state(&ctx.pool, "game-runtime")
        .await
        .expect("load runtime snapshot");
    let mod_paths = snapshot
        .current_mods
        .iter()
        .map(|member| member.mod_path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(mod_paths, vec!["AINOZ/Blue", "AINOZ/Red"]);
    assert!(snapshot.active_collection_id.is_none());
    assert!(snapshot.active_collection_name.is_none());
    assert!(!snapshot.is_safe);
    assert!(snapshot.is_dirty);
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Unsaved);
    assert_eq!(
        snapshot.last_changes.as_ref().map(|changes| changes.source),
        Some(LastChangesSource::Live)
    );
    assert_eq!(snapshot.missing_count, 0);
}
#[tokio::test]
async fn runtime_state_never_treats_last_changes_as_an_active_baseline() {
    let ctx = init_test_db().await;

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-1",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert game");

    let unsaved =
        collection::create(&ctx.pool, "unsaved-1", "game-1", "202603251217", true, true)
            .await
            .expect("create unsaved");
    collection::runtime::set_active(&ctx.pool, "game-1", Some(&unsaved.id))
        .await
        .expect("set active pointer");

    let snapshot = get_collection_runtime_state(&ctx.pool, "game-1")
        .await
        .expect("get runtime state");

    assert_eq!(snapshot.active_collection_id.as_deref(), None);
    assert_eq!(snapshot.active_collection_name.as_deref(), None);
}

#[tokio::test]
async fn runtime_descriptor_surfaces_draft_last_changes_without_a_preview_tree() {
    let ctx = init_test_db().await;

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-draft",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert game");
    let baseline = collection::create(
        &ctx.pool,
        "baseline-1",
        "game-draft",
        "Baseline",
        true,
        false,
    )
    .await
    .expect("create baseline");
    let draft = collection::create(
        &ctx.pool,
        "draft-1",
        "game-draft",
        "Last changes",
        true,
        true,
    )
    .await
    .expect("create draft");
    let mut tx = ctx.pool.begin().await.expect("begin runtime transaction");
    collection::runtime::set_draft_tx(&mut tx, "game-draft", &draft.id, Some(&baseline.id))
        .await
        .expect("set draft pointer");
    tx.commit().await.expect("commit runtime transaction");

    let descriptor = get_collection_runtime_descriptor(&ctx.pool, "game-draft")
        .await
        .expect("load runtime descriptor");

    assert_eq!(descriptor.runtime_status, RuntimeStatus::Unsaved);
    assert_eq!(descriptor.counts.active_mod_count, 0);
    assert_eq!(descriptor.missing_count, 0);
    assert_eq!(
        descriptor
            .last_changes
            .as_ref()
            .map(|changes| changes.source),
        Some(LastChangesSource::Draft)
    );
    assert_eq!(
        descriptor
            .last_changes
            .as_ref()
            .and_then(|changes| changes.base_collection_id.as_deref()),
        Some("baseline-1")
    );
}

#[tokio::test]
async fn runtime_state_counts_members_deleted_from_disk() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mod_dir = mods_root.path().join("AINOZ").join("Blue");
    std::fs::create_dir_all(&mod_dir).expect("create mod folder");
    std::fs::write(mod_dir.join("mod.ini"), "[TextureOverrideTest]\n").expect("write mod file");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-missing",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert game");
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-missing",
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-1",
            game_id: "game-missing",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert mod");
    create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-missing".to_string(),
            name: "Baseline".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save baseline");

    std::fs::remove_dir_all(&mod_dir).expect("delete mod folder");
    crate::services::collection::handle_mod_missing(
        &ctx.pool,
        "game-missing",
        "AINOZ/Blue",
    )
    .await
    .expect("record missing collection member");
    sqlx::query("DELETE FROM mods WHERE id = 'mod-1'")
        .execute(&ctx.pool)
        .await
        .expect("remove stale runtime row");

    let snapshot = get_collection_runtime_state(&ctx.pool, "game-missing")
        .await
        .expect("load runtime state");
    let descriptor = get_collection_runtime_descriptor(&ctx.pool, "game-missing")
        .await
        .expect("load runtime descriptor");
    assert_eq!(snapshot.missing_count, 1);
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Modified);
    assert_eq!(descriptor.missing_count, 1);
    assert_eq!(descriptor.runtime_status, RuntimeStatus::Modified);

    std::fs::create_dir_all(&mod_dir).expect("restore mod folder");
    std::fs::write(mod_dir.join("mod.ini"), "[TextureOverrideTest]\n").expect("restore mod file");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-1",
            game_id: "game-missing",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("restore runtime row");

    let restored = get_collection_runtime_state(&ctx.pool, "game-missing")
        .await
        .expect("load restored runtime state");
    let restored_descriptor = get_collection_runtime_descriptor(&ctx.pool, "game-missing")
        .await
        .expect("load restored runtime descriptor");
    assert_eq!(restored.missing_count, 0);
    assert_eq!(restored.runtime_status, RuntimeStatus::Clean);
    assert_eq!(restored_descriptor.missing_count, 0);
    assert_eq!(restored_descriptor.runtime_status, RuntimeStatus::Clean);
}
