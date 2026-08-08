use super::*;

#[tokio::test]
async fn partial_apply_blocks_when_mods_root_is_unavailable_even_when_ignoring_missing() {
    let ctx = init_test_db().await;
    let temp_root = tempfile::tempdir().expect("create temp root");
    let missing_root = temp_root.path().join("missing-mods-root");
    let mods_path = missing_root.to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let target_mod = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let target_object = test_collection_object(&collection.id);
    let projected_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&target_mod),
        std::slice::from_ref(&target_object),
        None,
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[target_mod],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    let result = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-1",
        collection_id: &collection.id,
        is_safe: true,
        mods_path: missing_root,
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: true,
        settings: AppSettings::default(),
    })
    .await;

    match result {
        Err(CollectionError::Corridor(crate::domain::errors::CorridorError::NoModsPath {
            game_id,
        })) => assert_eq!(game_id, "game-1"),
        other => panic!("expected source unavailable NoModsPath error, got {other:?}"),
    }
}

#[tokio::test]
async fn replace_collection_with_current_state_drops_missing_partial_apply_members() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
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
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert enabled mod");

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let target_mods = vec![
        test_collection_mod(&collection.id, "AINOZ/Blue", "Blue"),
        test_collection_mod(&collection.id, "AINOZ/Missing Mod", "Missing Mod"),
    ];
    let target_objects = vec![test_collection_object(&collection.id)];
    let projected_state = projected_state_service::build_projected_state(
        &target_mods,
        &target_objects,
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &target_mods,
        &target_objects,
        &projected_state,
    )
    .await
    .expect("persist collection state");

    apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-1",
        collection_id: &collection.id,
        is_safe: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: true,
        settings: AppSettings::default(),
    })
    .await
    .expect("partial apply succeeds");

    let updated = replace_collection_with_current_state(&ctx.pool, "game-1", &collection.id)
        .await
        .expect("replace original snapshot");
    let replaced_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load replaced collection mods");
    let replaced = collection_repo::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("load replaced collection")
        .expect("collection exists");
    let replaced_state = projected_state_service::parse_snapshot_json(
        replaced.snapshot_json.as_deref().expect("snapshot json"),
    )
    .expect("parse replaced snapshot");

    assert_eq!(updated.id, collection.id);
    assert_eq!(replaced_mods.len(), 1);
    assert_eq!(replaced_mods[0].mod_path, "AINOZ/Blue");
    assert_eq!(replaced_state.summary.missing_root_count, 0);
    assert_eq!(replaced_state.summary.active_root_count, 1);
}
