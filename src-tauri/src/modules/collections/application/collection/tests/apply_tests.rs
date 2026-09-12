use super::*;

#[tokio::test]
async fn collection_preflight_scope_includes_active_nonmembers_and_excludes_disabled_ones() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "active-nonmember",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Active Nonmember",
            folder_path: "AINOZ/Active Nonmember",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert active nonmember");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "disabled-unrelated",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Disabled Unrelated",
            folder_path: "AINOZ/DISABLED Disabled Unrelated",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert disabled unrelated mod");

    let collection = collection::create(&ctx.pool, "target", "game-1", "Target", true, false)
        .await
        .expect("create collection");
    let mods = vec![test_collection_mod(
        &collection.id,
        "AINOZ/Target Mod",
        "Target",
    )];
    let objects = vec![test_collection_object(&collection.id)];
    let projected_state = projected_state::build_projected_state(&mods, &objects, Some(&mods_path));
    persist_projected_state(&ctx.pool, &collection.id, &mods, &objects, &projected_state)
        .await
        .expect("persist collection state");

    let paths = collection_preflight_scope_paths(&ctx.pool, "game-1", "target", mods_root.path())
        .await
        .expect("build target paths");
    let normalized = paths
        .iter()
        .map(|path| path.replace('\\', "/"))
        .collect::<Vec<_>>();

    assert!(normalized.iter().any(|path| path.ends_with("/AINOZ")));
    assert!(normalized
        .iter()
        .any(|path| path.ends_with("/AINOZ/Target Mod")));
    assert!(normalized
        .iter()
        .any(|path| path.ends_with("/AINOZ/Active Nonmember")));
    assert!(!normalized
        .iter()
        .any(|path| path.ends_with("/AINOZ/DISABLED Disabled Unrelated")));
}

#[tokio::test]
async fn apply_collection_returns_missing_mods_before_disk_mutation_when_not_ignoring() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;

    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    std::fs::create_dir_all(mods_root.path().join("AINOZ")).expect("create object folder");

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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
        is_safe: true,
        safety_source: Some("manual".to_string()),
    };
    let object = CollectionObject {
        kind: MemberKind::Object,
        collection_id: collection.id.clone(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some("AINOZ".to_string()),
    };
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&missing_mod),
        std::slice::from_ref(&object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
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
        capture_last_changes: false,
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

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime");
    assert!(
        runtime
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

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let target_mods = vec![
        test_collection_mod(&collection.id, "AINOZ/Blue", "Blue"),
        test_collection_mod(&collection.id, "AINOZ/Missing Mod", "Missing Mod"),
    ];
    let target_objects = vec![test_collection_object(&collection.id)];
    let projected_state =
        projected_state::build_projected_state(&target_mods, &target_objects, Some(&mods_path));
    persist_projected_state(
        &ctx.pool,
        &collection.id,
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
        capture_last_changes: false,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: true,
        settings: AppSettings::default(),
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

    let original_mods = collection::get_mods(&ctx.pool, &collection.id)
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
async fn applying_a_collection_is_independent_of_its_safety_classification() {
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

    let collection = collection::create(
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
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&target_mod),
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[target_mod],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist unsafe collection state");

    let result = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-apply-no-mode",
        collection_id: &collection.id,
        capture_last_changes: false,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await
    .expect("safety classification must not block applying a collection");

    assert_eq!(result.mods_enabled, 1);
    let row: (String, i64) = sqlx::query_as(
        "SELECT folder_path, status FROM mods WHERE game_id = ? AND actual_name = ?",
    )
    .bind("game-apply-no-mode")
    .bind("Red")
    .fetch_one(&ctx.pool)
    .await
    .expect("load mod row");

    assert_eq!(row.0.replace('\\', "/"), "AINOZ/Red");
    assert_eq!(row.1, ItemStatus::Enabled as i64);
    assert!(!mods_root.path().join("AINOZ/DISABLED Red").exists());
    assert!(mods_root.path().join("AINOZ/Red").exists());
}

#[tokio::test]
async fn failed_parent_rename_rolls_back_the_child_and_marks_the_task_failed() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-partial-rename", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-partial-rename").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/DISABLED Red");
    std::fs::write(mods_root.path().join("DISABLED AINOZ"), b"collision")
        .expect("create deterministic parent rename collision");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-partial-rename",
            game_id: "game-partial-rename",
            object_id: Some("object-1"),
            actual_name: "Red",
            folder_path: "AINOZ/DISABLED Red",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert disabled mod");

    let collection = collection::create(
        &ctx.pool,
        "collection-partial-rename",
        "game-partial-rename",
        "Partial rename",
        true,
        false,
    )
    .await
    .expect("create collection");
    let target_mod = test_collection_mod(&collection.id, "AINOZ/Red", "Red");
    let target_object = CollectionObject {
        is_enabled: false,
        ..test_collection_object(&collection.id)
    };
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&target_mod),
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[target_mod],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    let result = apply_collection(ApplyCollectionRequest {
        pool: &ctx.pool,
        game_id: "game-partial-rename",
        collection_id: &collection.id,
        capture_last_changes: false,
        mods_path: mods_root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    })
    .await;

    assert!(result.is_err(), "parent collision must fail the apply");
    assert!(!mods_root.path().join("AINOZ/Red").exists());
    assert!(mods_root.path().join("AINOZ/DISABLED Red").is_dir());
    let mod_projection: (String, i64) = sqlx::query_as(
        "SELECT folder_path, status FROM mods WHERE game_id = ? AND actual_name = ?",
    )
    .bind("game-partial-rename")
    .bind("Red")
    .fetch_one(&ctx.pool)
    .await
    .expect("load reconciled mod");
    assert_eq!(mod_projection.0.replace('\\', "/"), "AINOZ/DISABLED Red");
    assert_eq!(mod_projection.1, ItemStatus::Disabled as i64);

    let task_status: String = sqlx::query_scalar(
        "SELECT status FROM tasks WHERE game_id = ? AND task_type = 'apply_collection'",
    )
    .bind("game-partial-rename")
    .fetch_one(&ctx.pool)
    .await
    .expect("load recovery task");
    assert_eq!(task_status, "FAILED");
}

#[tokio::test]
async fn restoring_a_draft_finalizes_its_baseline_with_the_apply_task() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();
    seed_game(&ctx.pool, "game-restore-finalize", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-restore-finalize").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Blue");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-restore-finalize",
            game_id: "game-restore-finalize",
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
    .expect("seed mod");

    let baseline = collection::create(
        &ctx.pool,
        "baseline-restore-finalize",
        "game-restore-finalize",
        "Baseline",
        true,
        false,
    )
    .await
    .expect("create baseline");
    let draft = collection::create(
        &ctx.pool,
        "draft-restore-finalize",
        "game-restore-finalize",
        "Last changes",
        true,
        true,
    )
    .await
    .expect("create draft");
    let target_mod = test_collection_mod(&draft.id, "AINOZ/Blue", "Blue");
    let target_object = test_collection_object(&draft.id);
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&target_mod),
        std::slice::from_ref(&target_object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &draft.id,
        &[target_mod],
        &[target_object],
        &projected_state,
    )
    .await
    .expect("persist draft");
    let mut connection = ctx.pool.acquire().await.expect("acquire connection");
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut connection,
        "game-restore-finalize",
        &draft.id,
        Some(&baseline.id),
    )
    .await
    .expect("set draft");
    drop(connection);

    crate::modules::collections::application::collection::restore_collection_with_baseline(
        ApplyCollectionRequest {
            pool: &ctx.pool,
            game_id: "game-restore-finalize",
            collection_id: &draft.id,
            capture_last_changes: false,
            mods_path: mods_root.path().to_path_buf(),
            suppressor: Arc::new(WatcherSuppressor::new(false)),
            ignore_missing: false,
            settings: AppSettings::default(),
        },
        Some(baseline.id.clone()),
    )
    .await
    .expect("restore draft");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(
        &ctx.pool,
        "game-restore-finalize",
    )
    .await
    .expect("load runtime")
    .expect("runtime exists");
    assert_eq!(runtime.active_collection_id, Some(baseline.id));
    let task_status: String =
        sqlx::query_scalar("SELECT status FROM tasks WHERE game_id = ? AND target_id = ?")
            .bind("game-restore-finalize")
            .bind(&draft.id)
            .fetch_one(&ctx.pool)
            .await
            .expect("load restore task");
    assert_eq!(task_status, "COMPLETED");
}
