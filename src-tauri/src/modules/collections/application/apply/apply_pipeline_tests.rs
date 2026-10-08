use super::*;

#[tokio::test]
async fn successful_apply_surfaces_task_finalization_failure() {
    let test_db = crate::test_utils::init_test_db().await;
    let pool = test_db.pool.clone();
    pool.close().await;
    let mut apply_context = ApplyContext::new(ApplyCollectionRequest {
        pool: &pool,
        game_id: "game-1",
        collection_id: "collection-1",
        capture_last_changes: false,
        mods_path: PathBuf::from("E:/Mods"),
        suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    });
    apply_context.final_state_name = Some("Preset".to_string());

    let error = finalize_apply(&apply_context, "task-1", TaskStatus::Pending)
        .await
        .expect_err("task finalization failure must be returned");

    assert!(matches!(error, CollectionError::Db(_)));
}

#[tokio::test]
async fn finalization_failure_rolls_back_active_baseline_and_task_completion_together() {
    let test_db = crate::test_utils::init_test_db().await;
    crate::test_utils::insert_test_game(
        &test_db.pool,
        &crate::test_utils::TestGameFixture {
            id: "game-atomic-finalize",
            name: "Atomic finalize",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "E:/Games/Atomic",
            mods_path: Some("E:/Mods/Atomic"),
        },
    )
    .await
    .expect("seed game");
    for (id, name) in [("baseline-before", "Before"), ("baseline-after", "After")] {
        crate::modules::collections::adapters::sqlite::create(
            &test_db.pool,
            id,
            "game-atomic-finalize",
            name,
            true,
            false,
        )
        .await
        .expect("seed collection");
    }
    crate::modules::collections::adapters::sqlite::runtime::set_active(
        &test_db.pool,
        "game-atomic-finalize",
        Some("baseline-before"),
    )
    .await
    .expect("seed active baseline");
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task(
        &test_db.pool,
        "task-atomic-finalize",
        "game-atomic-finalize",
        crate::modules::workspace::domain::task::TASK_TYPE_APPLY_COLLECTION,
        Some("baseline-after"),
    )
    .await
    .expect("seed running task");
    sqlx::query(
        "CREATE TRIGGER fail_task_completion BEFORE UPDATE OF status ON tasks \
         WHEN NEW.id = 'task-atomic-finalize' AND NEW.status = 'COMPLETED' \
         BEGIN SELECT RAISE(FAIL, 'injected task completion failure'); END",
    )
    .execute(&test_db.pool)
    .await
    .expect("install failure trigger");

    let mut apply_context = ApplyContext::new(ApplyCollectionRequest {
        pool: &test_db.pool,
        game_id: "game-atomic-finalize",
        collection_id: "baseline-after",
        capture_last_changes: false,
        mods_path: PathBuf::from("E:/Mods/Atomic"),
        suppressor: std::sync::Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings: AppSettings::default(),
    });
    apply_context.finalize_active_collection = true;
    apply_context.final_active_collection_id = Some("baseline-after".to_string());

    let error = finalize_apply(&apply_context, "task-atomic-finalize", TaskStatus::Running)
        .await
        .expect_err("injected finalization failure");
    assert!(matches!(error, CollectionError::Db(_)));

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(
        &test_db.pool,
        "game-atomic-finalize",
    )
    .await
    .expect("load runtime")
    .expect("runtime exists");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some("baseline-before"),
        "active baseline update must roll back with failed task completion"
    );
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(
        &test_db.pool,
        "task-atomic-finalize",
    )
    .await
    .expect("load task")
    .expect("task exists");
    assert_eq!(task.status, TaskStatus::Running);
}
