use super::*;

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

    let collections = collection_repo::list_for_game(&ctx.pool, "game-1")
        .await
        .expect("list collections");
    assert!(collections.is_empty());
}

#[tokio::test]
async fn saving_last_changes_as_a_collection_consumes_the_draft() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let draft = collection_repo::create(&ctx.pool, "draft-1", "game-1", "Last changes", true, true)
        .await
        .expect("create draft");
    let state = projected_state_service::empty_projected_state();
    persist_projected_state(&ctx.pool, &draft.id, &[], &[], &state)
        .await
        .expect("persist draft state");
    let mut tx = ctx.pool.begin().await.expect("begin runtime transaction");
    crate::repo::collection_runtime_repo::set_draft_tx(&mut tx, "game-1", &draft.id, None)
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

    let runtime = crate::repo::collection_runtime_repo::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert!(runtime.draft_collection_id.is_none());
    assert!(collection_repo::get_by_id(&ctx.pool, &draft.id)
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
    let draft = collection_repo::create(
        &ctx.pool,
        "draft-save-protected",
        "game-save-protected",
        "Last changes",
        true,
        true,
    )
    .await
    .expect("create draft");
    crate::repo::collection_runtime_repo::set_draft_tx(
        &mut ctx.pool.acquire().await.expect("runtime connection"),
        "game-save-protected",
        &draft.id,
        None,
    )
    .await
    .expect("set draft pointer");
    crate::repo::task_repo::create_task_with_rollback_intent(
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
    assert!(crate::repo::task_repo::compare_and_set_status(
        &ctx.pool,
        "open-save-rollback",
        crate::domain::task::TaskStatus::Pending,
        crate::domain::task::TaskStatus::Running,
    )
    .await
    .expect("claim open task"));

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
    let runtime = crate::repo::collection_runtime_repo::get(&ctx.pool, "game-save-protected")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(
        runtime.draft_collection_id.as_deref(),
        Some(draft.id.as_str())
    );
    assert!(collection_repo::get_by_id(&ctx.pool, &draft.id)
        .await
        .expect("load protected draft")
        .is_some());
}
