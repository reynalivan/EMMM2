use super::*;

#[tokio::test]
async fn apply_collection_returns_missing_mods_before_disk_mutation_when_not_ignoring() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;

    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
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
        std::slice::from_ref(&missing_mod),
        std::slice::from_ref(&object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[missing_mod],
        &[object],
        &projected_state,
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
        reconcile_lock: None,
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
async fn partial_apply_skips_missing_paths_without_replacing_original_collection() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;

    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Blue");
    create_flat_mod_folder(mods_root.path(), "AINOZ/Green");

    for (id, name, folder_path) in [
        ("mod-blue", "Blue", "AINOZ/Blue"),
        ("mod-green", "Green", "AINOZ/Green"),
    ] {
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id,
                game_id: "game-1",
                object_id: Some("object-1"),
                actual_name: name,
                folder_path,
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(&mods_path),
            },
        )
        .await
        .expect("insert enabled mod");
    }

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

    let result = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-1",
        collection_id: &collection.id,
        is_safe: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: true,
        settings: AppSettings::default(),
        reconcile_lock: None,
    })
    .await
    .expect("partial apply succeeds");

    assert!(result.partial_apply);
    assert_eq!(result.skipped_missing_paths, vec!["AINOZ/Missing Mod"]);
    assert_eq!(result.mods_disabled, 1);
    assert_eq!(result.runtime_path_rewrites.len(), 1);
    assert_eq!(
        result.runtime_path_rewrites[0].old_path.replace('\\', "/"),
        mods_root
            .path()
            .join("AINOZ")
            .join("Green")
            .to_string_lossy()
            .to_string()
            .replace('\\', "/")
    );
    assert_eq!(
        result.runtime_path_rewrites[0].new_path.replace('\\', "/"),
        mods_root
            .path()
            .join("AINOZ")
            .join("DISABLED Green")
            .to_string_lossy()
            .to_string()
            .replace('\\', "/")
    );

    let original_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load original collection mods");
    assert_eq!(
        original_mods
            .iter()
            .map(|entry| entry.mod_path.as_str())
            .collect::<Vec<_>>(),
        vec!["AINOZ/Blue", "AINOZ/Missing Mod"]
    );
}

#[tokio::test]
async fn apply_collection_rejects_cross_corridor_request() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-apply-no-mode", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-apply-no-mode").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/DISABLED Red");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-apply-no-mode",
            game_id: "game-apply-no-mode",
            object_id: Some("object-1"),
            actual_name: "Red",
            folder_path: "AINOZ/DISABLED Red",
            status: ItemStatus::Disabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert disabled unsafe mod");

    let collection = collection_repo::create(
        &ctx.pool,
        "unsafe-collection",
        "game-apply-no-mode",
        "Unsafe Preset",
        false,
        false,
    )
    .await
    .expect("create unsafe collection");
    let target_mod = test_collection_mod(&collection.id, "AINOZ/Red", "Red");
    let target_object = test_collection_object(&collection.id);
    let projected_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&target_mod),
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        false,
        &[target_mod],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist unsafe collection state");

    // Corridor enforcement: applying an UNSAFE collection while the request is
    // in the SAFE corridor must be rejected before any filesystem mutation.
    let result = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-apply-no-mode",
        collection_id: &collection.id,
        is_safe: true,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
        reconcile_lock: None,
    })
    .await;

    assert!(
        matches!(result, Err(CollectionError::Validation(_))),
        "cross-corridor apply must be rejected, got {result:?}"
    );

    // The mod stays disabled on disk and in the DB — no mutation occurred.
    let row: (String, i64) = sqlx::query_as("SELECT folder_path, status FROM mods WHERE id = ?")
        .bind("mod-apply-no-mode")
        .fetch_one(&ctx.pool)
        .await
        .expect("load mod row");

    assert_eq!(row.0.replace('\\', "/"), "AINOZ/DISABLED Red");
    assert_eq!(row.1, ItemStatus::Disabled as i64);
    assert!(mods_root.path().join("AINOZ/DISABLED Red").exists());
    assert!(!mods_root.path().join("AINOZ/Red").exists());
}
