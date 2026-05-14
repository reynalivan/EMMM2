use super::{
    apply_collection, create_collection, delete_collection, get_collection_preview,
    handle_dirty_state, handle_mod_moved_or_renamed, preview_apply, update_collection,
    ApplyCollectionRequest,
};
use crate::database::models::{GameType, ItemStatus};
use crate::domain::collection::{
    CollectionMod, CollectionObject, CreateCollectionInput, CreateCollectionMode, MemberKind,
    ProjectedCollectionState, ProjectedStateSummary, UpdateCollectionInput,
};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::config::AppSettings;
use crate::services::projected_state_service;
use crate::services::scanner::watcher::WatcherSuppressor;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};
use std::sync::Arc;

#[tokio::test]
async fn delete_collection_promotes_corridor_unsaved_when_active_is_removed() {
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

    let active =
        collection_repo::create(&ctx.pool, "named-1", "game-1", "Named Preset", true, false)
            .await
            .expect("create active");
    let unsaved = collection_repo::create(
        &ctx.pool,
        "unsaved-1",
        "game-1",
        "Unsaved 202603251200",
        true,
        true,
    )
    .await
    .expect("create unsaved");
    let undo = collection_repo::create(&ctx.pool, "undo-1", "game-1", "Undo Snapshot", true, false)
        .await
        .expect("create undo");

    corridor_repo::update_pointers(&ctx.pool, "game-1", true, Some(&active.id), Some(&undo.id))
        .await
        .expect("set pointers");

    delete_collection(&ctx.pool, &active.id)
        .await
        .expect("delete active");

    let snapshot = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor")
        .expect("corridor exists");

    assert_eq!(
        snapshot.active_collection_id.as_deref(),
        Some(unsaved.id.as_str())
    );
    assert!(snapshot.undo_collection_id.is_none());
}

#[tokio::test]
async fn delete_saved_active_collection_recreates_unsaved_and_marks_it_active() {
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

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-1",
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
            game_id: "game-1",
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
    .expect("insert mod");

    let initial_unsaved = handle_dirty_state(&ctx.pool, "game-1", true)
        .await
        .expect("create unsaved");
    assert!(initial_unsaved.is_unsaved);
    assert!(initial_unsaved.is_active);

    let named = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Named Preset".to_string(),
            is_safe: true,
            save_mode: None,
            source_collection_id: None,
        },
    )
    .await
    .expect("create named collection");
    assert!(!named.is_unsaved);
    assert!(named.is_active);

    let unsaved_after_save =
        collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
            .await
            .expect("query unsaved after save");
    assert!(unsaved_after_save.is_none());

    delete_collection(&ctx.pool, &named.id)
        .await
        .expect("delete named collection");

    let corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor")
        .expect("corridor exists");
    let recreated_unsaved =
        collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
            .await
            .expect("query recreated unsaved")
            .expect("unsaved recreated");
    let collections = collection_repo::list_for_corridor(&ctx.pool, "game-1", true, true)
        .await
        .expect("list collections");

    assert_eq!(
        corridor.active_collection_id.as_deref(),
        Some(recreated_unsaved.id.as_str())
    );
    assert_eq!(collections.len(), 1);
    assert!(recreated_unsaved.is_unsaved);
    assert_ne!(recreated_unsaved.id, initial_unsaved.id);
}

#[tokio::test]
async fn clone_snapshot_keeps_existing_unsaved_and_active_pointer() {
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

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-1",
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
            game_id: "game-1",
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
    .expect("insert mod");

    let unsaved = handle_dirty_state(&ctx.pool, "game-1", true)
        .await
        .expect("create unsaved");

    let cloned = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Cloned Preset".to_string(),
            is_safe: true,
            save_mode: Some(CreateCollectionMode::CloneSnapshot),
            source_collection_id: Some(unsaved.id.clone()),
        },
    )
    .await
    .expect("clone unsaved snapshot");

    let corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor")
        .expect("corridor exists");
    let unsaved_after_clone =
        collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
            .await
            .expect("query unsaved after clone")
            .expect("unsaved still exists");

    assert_eq!(
        corridor.active_collection_id.as_deref(),
        Some(unsaved.id.as_str())
    );
    assert_eq!(unsaved_after_clone.id, unsaved.id);
    assert!(!cloned.is_active);
    assert!(!cloned.is_unsaved);
}

#[tokio::test]
async fn update_collection_returns_preview_tree_mod_count() {
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

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Original", true, false)
            .await
            .expect("create collection");

    let snapshot = ProjectedCollectionState {
        object_states: Vec::new(),
        active_roots: Vec::new(),
        summary: ProjectedStateSummary {
            object_count: 0,
            enabled_object_count: 0,
            active_root_count: 7,
            missing_root_count: 0,
        },
    };
    let snapshot_json =
        projected_state_service::serialize_snapshot_json(&snapshot).expect("serialize snapshot");

    sqlx::query(
        "UPDATE collections SET snapshot_json = ?, signature = ?, root_count = ? WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind("sig-1")
    .bind(7_i32)
    .bind(&collection.id)
    .execute(&ctx.pool)
    .await
    .expect("update snapshot");

    let updated = update_collection(
        &ctx.pool,
        UpdateCollectionInput {
            id: collection.id.clone(),
            game_id: "game-1".to_string(),
            name: Some("Renamed".to_string()),
        },
    )
    .await
    .expect("update collection");

    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.mod_count, 7);
}

#[tokio::test]
async fn delete_in_other_corridor_does_not_create_unsaved_here() {
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

    let unsafe_named = collection_repo::create(
        &ctx.pool,
        "unsafe-named",
        "game-1",
        "Unsafe Named",
        false,
        false,
    )
    .await
    .expect("create unsafe named");

    corridor_repo::update_pointers(&ctx.pool, "game-1", false, Some(&unsafe_named.id), None)
        .await
        .expect("set unsafe active");
    corridor_repo::update_pointers(&ctx.pool, "game-1", true, None, None)
        .await
        .expect("seed safe corridor");

    delete_collection(&ctx.pool, &unsafe_named.id)
        .await
        .expect("delete unsafe named");

    let safe_unsaved = collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
        .await
        .expect("query safe unsaved");
    let safe_corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load safe corridor")
        .expect("safe corridor exists");

    assert!(safe_unsaved.is_none());
    assert!(safe_corridor.active_collection_id.is_none());
}

#[tokio::test]
async fn apply_collection_returns_missing_mods_before_disk_mutation_when_not_ignoring() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-1",
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
            game_id: "game-1",
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");
    std::fs::create_dir_all(mods_root.path().join("AINOZ")).expect("create object folder");

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let missing_mod = CollectionMod {
        kind: MemberKind::Mod,
        collection_id: collection.id.clone(),
        mod_id: None,
        mod_path: "AINOZ/Missing Mod".to_string(),
        mod_path_key: Some("ainoz/missing mod".to_string()),
        object_id: "object-1".to_string(),
        display_name: Some("Missing Mod".to_string()),
        preview_path: Some("AINOZ/Missing Mod".to_string()),
        node_type: Some("FlatModRoot".to_string()),
        warnings: Vec::new(),
        is_enabled: true,
    };
    let object = CollectionObject {
        kind: MemberKind::Object,
        collection_id: collection.id.clone(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some("AINOZ".to_string()),
    };
    let projected_state = projected_state_service::build_projected_state(
        &[missing_mod.clone()],
        &[object.clone()],
        Some(&mods_path),
    );
    let roots =
        projected_state_service::roots_from_projected_state(&collection.id, true, &projected_state);

    collection_repo::replace_all_state(
        &ctx.pool,
        &collection.id,
        &[missing_mod],
        &[object],
        &roots,
        Some(&projected_state_service::signature_for_projected_state(
            &projected_state,
        )),
        projected_state_service::serialize_snapshot_json(&projected_state).as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await
    .expect("persist collection state");

    let result = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-1",
        collection_id: &collection.id,
        is_safe: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await;

    match result {
        Err(CollectionError::MissingMods { count, paths }) => {
            assert_eq!(count, 1);
            assert_eq!(paths, vec!["AINOZ/Missing Mod".to_string()]);
        }
        other => panic!("expected MissingMods error, got {other:?}"),
    }

    let corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor");
    assert!(
        corridor
            .and_then(|state| state.active_collection_id)
            .is_none(),
        "missing target must fail before setting active collection"
    );
}

#[tokio::test]
async fn preview_apply_rejects_cross_corridor_collection() {
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

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Unsafe", false, false)
            .await
            .expect("create collection");

    let result = preview_apply(&ctx.pool, "game-1", &collection.id, true, Some("E:/Mods")).await;

    match result {
        Err(CollectionError::Validation(message)) => {
            assert!(message.contains("requested corridor"));
        }
        other => panic!("expected corridor validation error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_collection_preview_rejects_cross_game_collection() {
    let ctx = init_test_db().await;
    for game_id in ["game-1", "game-2"] {
        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: game_id,
                name: "Test Game",
                game_type: GameType::GIMI,
                path: if game_id == "game-1" {
                    "E:/Games/TestGame1"
                } else {
                    "E:/Games/TestGame2"
                },
                mods_path: if game_id == "game-1" {
                    Some("E:/Mods1")
                } else {
                    Some("E:/Mods2")
                },
            },
        )
        .await
        .expect("insert game");
    }

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");

    let result = get_collection_preview(&ctx.pool, "game-2", &collection.id, Some("E:/Mods")).await;

    match result {
        Err(CollectionError::Validation(message)) => {
            assert!(message.contains("does not belong to game"));
        }
        other => panic!("expected game validation error, got {other:?}"),
    }
}

#[tokio::test]
async fn auto_heal_rebuilds_snapshot_roots_signature_and_path_keys() {
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
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-1",
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let old_mod = CollectionMod {
        kind: MemberKind::Mod,
        collection_id: collection.id.clone(),
        mod_id: None,
        mod_path: "AINOZ/Old Mod".to_string(),
        mod_path_key: Some(crate::services::path_key::folder_path_key(
            "AINOZ/Old Mod",
            None,
        )),
        object_id: "object-1".to_string(),
        display_name: Some("Old Mod".to_string()),
        preview_path: Some("AINOZ/Old Mod".to_string()),
        node_type: None,
        warnings: Vec::new(),
        is_enabled: true,
    };
    let object = CollectionObject {
        kind: MemberKind::Object,
        collection_id: collection.id.clone(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some("AINOZ".to_string()),
    };
    let old_state = projected_state_service::build_projected_state(
        &[old_mod.clone()],
        &[object.clone()],
        Some("E:/Mods"),
    );
    let old_roots =
        projected_state_service::roots_from_projected_state(&collection.id, true, &old_state);
    collection_repo::replace_all_state(
        &ctx.pool,
        &collection.id,
        &[old_mod],
        &[object],
        &old_roots,
        Some(&projected_state_service::signature_for_projected_state(
            &old_state,
        )),
        projected_state_service::serialize_snapshot_json(&old_state).as_deref(),
        old_state.summary.active_root_count as i32,
    )
    .await
    .expect("persist old state");

    handle_mod_moved_or_renamed(&ctx.pool, "AINOZ/Old Mod", "AINOZ/New Mod", None)
        .await
        .expect("auto heal path");

    let healed = collection_repo::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("load collection")
        .expect("collection exists");
    let healed_state = projected_state_service::parse_snapshot_json(
        healed.snapshot_json.as_deref().expect("snapshot json"),
    )
    .expect("parse healed snapshot");
    let healed_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load healed mods");
    let healed_roots = collection_repo::get_roots(&ctx.pool, &collection.id)
        .await
        .expect("load healed roots");
    let expected_key = crate::services::path_key::folder_path_key("AINOZ/New Mod", None);

    assert_eq!(
        healed_mods
            .first()
            .and_then(|mod_row| mod_row.mod_path_key.as_deref()),
        Some(expected_key.as_str())
    );
    assert_eq!(
        healed_state
            .active_roots
            .first()
            .map(|root| root.source_path.as_str()),
        Some("AINOZ/New Mod")
    );
    assert_eq!(
        healed_roots.first().map(|root| root.root_path.as_str()),
        Some("AINOZ/New Mod")
    );
    assert_eq!(
        healed.display_mod_count,
        healed_state.summary.active_root_count as i32
    );
}
