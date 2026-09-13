use super::*;

#[tokio::test]
async fn passive_snapshot_reuses_an_identical_named_collection() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-passive-reuse", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-passive-reuse", "game-passive-reuse").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-passive-reuse",
            game_id: "game-passive-reuse",
            object_id: Some("object-passive-reuse"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert active mod");
    let saved = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-passive-reuse".to_string(),
            name: "Existing state".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save existing state");

    let backup = snapshot_live_state_passively(&ctx.pool, "game-passive-reuse", "Backup")
        .await
        .expect("reuse identical state");

    assert!(!backup.created);
    assert_eq!(backup.collection_id, saved.id);
    assert_eq!(backup.collection_name, "Existing state");
    assert_eq!(
        collection::list_for_game(&ctx.pool, "game-passive-reuse")
            .await
            .expect("list collections")
            .len(),
        1
    );
}

#[tokio::test]
async fn passive_snapshot_creates_a_suffixed_name_without_changing_runtime_state() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-passive-create", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-passive-create", "game-passive-create").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-passive-create",
            game_id: "game-passive-create",
            object_id: Some("object-passive-create"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert active mod");
    let baseline = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-passive-create".to_string(),
            name: "Baseline".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save baseline");
    collection::create(
        &ctx.pool,
        "existing-backup-name",
        "game-passive-create",
        "Backup",
        true,
        false,
    )
    .await
    .expect("create name collision");
    collection::create(
        &ctx.pool,
        "existing-backup-suffix",
        "game-passive-create",
        "Backup (2)",
        true,
        false,
    )
    .await
    .expect("create suffixed name collision");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-passive-create-green",
            game_id: "game-passive-create",
            object_id: Some("object-passive-create"),
            actual_name: "Green",
            folder_path: "AINOZ/Green",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("change live state");

    let backup = snapshot_live_state_passively(&ctx.pool, "game-passive-create", "Backup")
        .await
        .expect("save distinct state");

    assert!(backup.created);
    assert_eq!(backup.collection_name, "Backup (3)");
    assert_eq!(
        collection::get_mods(&ctx.pool, &backup.collection_id)
            .await
            .expect("load backup members")
            .len(),
        2
    );
    let runtime = collection::runtime::get(&ctx.pool, "game-passive-create")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some(baseline.id.as_str())
    );
    assert!(runtime.draft_collection_id.is_none());
}

#[tokio::test]
async fn passive_snapshot_rejects_an_empty_live_state() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-passive-empty", Some("E:/Mods")).await;

    let error = snapshot_live_state_passively(&ctx.pool, "game-passive-empty", "Backup")
        .await
        .expect_err("empty state must not be saved");

    assert!(matches!(error, CollectionError::Validation(_)));
    assert!(collection::list_for_game(&ctx.pool, "game-passive-empty")
        .await
        .expect("list collections")
        .is_empty());
}

#[tokio::test]
async fn create_collection_rolls_back_row_when_snapshot_persistence_fails() {
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
    sqlx::query(
        r#"CREATE TRIGGER fail_collection_mod_insert
        BEFORE INSERT ON collection_mods
        BEGIN
            SELECT RAISE(ABORT, 'forced snapshot persistence failure');
        END"#,
    )
    .execute(&ctx.pool)
    .await
    .expect("create failure trigger");

    create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Atomic Preset".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect_err("snapshot failure must fail create");

    let collections = collection::list_for_game(&ctx.pool, "game-1")
        .await
        .expect("list collections");
    assert!(collections.is_empty());
}

#[tokio::test]
async fn saving_last_changes_as_a_collection_consumes_the_draft() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let draft = collection::create(&ctx.pool, "draft-1", "game-1", "Last changes", true, true)
        .await
        .expect("create draft");
    let state = projected_state::empty_projected_state();
    persist_projected_state(&ctx.pool, &draft.id, &[], &[], &state)
        .await
        .expect("persist draft state");
    let mut tx = ctx.pool.begin().await.expect("begin runtime transaction");
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut tx, "game-1", &draft.id, None,
    )
    .await
    .expect("set draft pointer");
    tx.commit().await.expect("commit runtime transaction");

    let saved = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Recovered changes".to_string(),
            save_mode: Some(CreateCollectionMode::CloneSnapshot),
            source_collection_id: Some(draft.id.clone()),
        },
    )
    .await
    .expect("save draft as named collection");
    assert_eq!(saved.name, "Recovered changes");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert!(runtime.draft_collection_id.is_none());
    assert!(collection::get_by_id(&ctx.pool, &draft.id)
        .await
        .expect("query consumed draft")
        .is_none());
}

#[tokio::test]
async fn save_current_rejects_a_draft_referenced_by_an_open_apply() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-save-protected", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-save-protected", "game-save-protected").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-save-protected",
            game_id: "game-save-protected",
            object_id: Some("object-save-protected"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert active mod");
    let draft = collection::create(
        &ctx.pool,
        "draft-save-protected",
        "game-save-protected",
        "Last changes",
        true,
        true,
    )
    .await
    .expect("create draft");
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut ctx.pool.acquire().await.expect("runtime connection"),
        "game-save-protected",
        &draft.id,
        None,
    )
    .await
    .expect("set draft pointer");
    crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
        &ctx.pool,
        "open-save-rollback",
        "game-save-protected",
        "apply_collection",
        Some("target"),
        Some(&draft.id),
        None,
    )
    .await
    .expect("create open task");
    assert!(
        crate::modules::workspace::adapters::sqlite::task::compare_and_set_status(
            &ctx.pool,
            "open-save-rollback",
            crate::modules::workspace::domain::task::TaskStatus::Pending,
            crate::modules::workspace::domain::task::TaskStatus::Running,
        )
        .await
        .expect("claim open task")
    );

    let error = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-save-protected".to_string(),
            name: "Unsafe save".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect_err("save current must retain a referenced rollback draft");

    assert!(format!("{error}").contains("recovery"));
    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(
        &ctx.pool,
        "game-save-protected",
    )
    .await
    .expect("load runtime")
    .expect("runtime exists");
    assert_eq!(
        runtime.draft_collection_id.as_deref(),
        Some(draft.id.as_str())
    );
    assert!(collection::get_by_id(&ctx.pool, &draft.id)
        .await
        .expect("load protected draft")
        .is_some());
}
