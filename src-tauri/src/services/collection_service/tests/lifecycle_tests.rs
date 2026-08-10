use super::*;

#[tokio::test]
async fn save_current_state_creates_snapshot_without_touching_legacy_corridor_pointers() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-save-no-pointer", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-save-no-pointer", "game-save-no-pointer").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-save-no-pointer",
            game_id: "game-save-no-pointer",
            object_id: Some("object-save-no-pointer"),
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
    let previous_active = collection_repo::create(
        &ctx.pool,
        "previous-active",
        "game-save-no-pointer",
        "Previous",
        true,
        false,
    )
    .await
    .expect("create previous active");
    set_test_corridor_active_unchecked(
        &ctx.pool,
        "game-save-no-pointer",
        true,
        Some(&previous_active.id),
    )
    .await
    .expect("seed legacy pointer");

    let saved = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-save-no-pointer".to_string(),
            name: "Saved Snapshot".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save current state");
    let corridor = corridor_repo::get(&ctx.pool, "game-save-no-pointer", true)
        .await
        .expect("load corridor")
        .expect("corridor row exists");

    assert!(!saved.is_unsaved);
    assert!(!saved.is_active);
    assert_eq!(
        corridor.active_collection_id.as_deref(),
        Some(previous_active.id.as_str())
    );
    assert!(collection_repo::find_unsaved_for_corridor(
        &ctx.pool,
        "game-save-no-pointer",
        true,
        None
    )
    .await
    .expect("query unsaved")
    .is_none());
}

#[tokio::test]
async fn list_collections_returns_all_named_presets_across_safety_flags() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-list-all", Some("E:/Mods")).await;
    collection_repo::create(
        &ctx.pool,
        "safe-collection",
        "game-list-all",
        "Safe",
        true,
        false,
    )
    .await
    .expect("create safe collection");
    collection_repo::create(
        &ctx.pool,
        "unsafe-collection",
        "game-list-all",
        "Unsafe",
        false,
        false,
    )
    .await
    .expect("create unsafe collection");
    collection_repo::create(
        &ctx.pool,
        "unsaved-collection",
        "game-list-all",
        "Unsaved",
        false,
        true,
    )
    .await
    .expect("create legacy unsaved collection");

    // Corridor enforcement: listing in the SAFE corridor returns only the safe
    // named collection — the unsafe and unsaved rows are excluded.
    let safe_collections = list_collections(
        &ctx.pool,
        "game-list-all",
        crate::domain::corridor::Corridor::from_is_safe(true),
    )
    .await
    .expect("list safe collections");
    let safe_ids = safe_collections
        .iter()
        .map(|collection| collection.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(safe_ids, vec!["safe-collection"]);

    // The UNSAFE corridor returns only the unsafe named collection.
    let unsafe_collections = list_collections(
        &ctx.pool,
        "game-list-all",
        crate::domain::corridor::Corridor::from_is_safe(false),
    )
    .await
    .expect("list unsafe collections");
    let unsafe_ids = unsafe_collections
        .iter()
        .map(|collection| collection.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(unsafe_ids, vec!["unsafe-collection"]);
}

#[tokio::test]
async fn dirty_state_refresh_returns_synthetic_runtime_without_unsaved_collection() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    let unsaved = collection_repo::create(&ctx.pool, "unsaved-1", "game-1", "Unsaved", false, true)
        .await
        .expect("create zero-mod unsaved");
    set_test_corridor_active_unchecked(&ctx.pool, "game-1", false, Some(&unsaved.id))
        .await
        .expect("activate unsaved");

    create_flat_mod_folder(mods_root.path(), "AINOZ/Blue");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-blue",
            game_id: "game-1",
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
    .expect("insert enabled unsafe mod");

    let summary = handle_dirty_state(&ctx.pool, "game-1")
        .await
        .expect("refresh dirty state");

    assert_eq!(summary.id, "__current_runtime__");
    assert_eq!(summary.mod_count, 1);
    assert!(summary.is_unsaved);
    assert!(!summary.is_active);
    assert!(collection_repo::get_by_id(&ctx.pool, &summary.id)
        .await
        .expect("query synthetic summary")
        .is_none());
    let stored_unsaved = collection_repo::get_by_id(&ctx.pool, &unsaved.id)
        .await
        .expect("query legacy unsaved")
        .expect("legacy unsaved remains untouched");
    assert_eq!(stored_unsaved.root_count, 0);
}

#[tokio::test]
async fn clone_snapshot_does_not_touch_legacy_active_pointer() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

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

    let source = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Source Preset".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("create source snapshot");
    set_test_corridor_active_unchecked(&ctx.pool, "game-1", true, Some(&source.id))
        .await
        .expect("seed legacy active pointer");

    let cloned = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Cloned Preset".to_string(),
            save_mode: Some(CreateCollectionMode::CloneSnapshot),
            source_collection_id: Some(source.id.clone()),
        },
    )
    .await
    .expect("clone source snapshot");

    let corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor")
        .expect("corridor exists");

    assert_eq!(
        corridor.active_collection_id.as_deref(),
        Some(source.id.as_str())
    );
    assert!(!cloned.is_active);
    assert!(!cloned.is_unsaved);
}

#[tokio::test]
async fn update_collection_returns_preview_tree_mod_count() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

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
