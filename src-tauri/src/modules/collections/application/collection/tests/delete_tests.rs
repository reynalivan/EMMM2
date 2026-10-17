use super::*;
use crate::modules::workspace::domain::task::{TaskStatus, TASK_TYPE_APPLY_COLLECTION};

#[tokio::test]
async fn collection_regression_preserves_open_task_rollback_snapshot() {
    for status in [TaskStatus::Pending, TaskStatus::Running] {
        let ctx = init_test_db().await;
        seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
        seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
        let saved = collection::create(&ctx.pool, "rollback", "game-1", "Baseline", true, false)
            .await
            .unwrap();
        let member = test_collection_mod(&saved.id, "AINOZ/Stored", "Stored");
        let object = test_collection_object(&saved.id);
        let state = projected_state::build_projected_state(
            std::slice::from_ref(&member),
            std::slice::from_ref(&object),
            Some("E:/Mods"),
        );
        persist_projected_state(&ctx.pool, &saved.id, &[member], &[object], &state)
            .await
            .unwrap();
        collection::runtime::set_active(&ctx.pool, "game-1", Some(&saved.id))
            .await
            .unwrap();
        crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
            &ctx.pool,
            "open-task",
            "game-1",
            TASK_TYPE_APPLY_COLLECTION,
            Some("target"),
            Some(&saved.id),
            Some(&saved.id),
        )
        .await
        .unwrap();
        if status == TaskStatus::Running {
            assert!(
                crate::modules::workspace::adapters::sqlite::task::compare_and_set_status(
                    &ctx.pool,
                    "open-task",
                    TaskStatus::Pending,
                    status.clone(),
                )
                .await
                .unwrap()
            );
        }
        let before = collection::get_by_id(&ctx.pool, &saved.id)
            .await
            .unwrap()
            .unwrap();

        let error = delete_collection(&ctx.pool, &saved.id)
            .await
            .expect_err("an open task's rollback snapshot must not be deleted");
        assert!(
            matches!(error, CollectionError::Validation(ref message) if message.contains("open recovery task"))
        );
        let after = collection::get_by_id(&ctx.pool, &saved.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.snapshot_json, before.snapshot_json);
        assert_eq!(
            collection::get_mods(&ctx.pool, &saved.id)
                .await
                .unwrap()
                .len(),
            1
        );
        let runtime = collection::runtime::get(&ctx.pool, "game-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            runtime.active_collection_id.as_deref(),
            Some(saved.id.as_str())
        );
        let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(
            &ctx.pool,
            "open-task",
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(task.status, status);
        assert_eq!(
            task.rollback_collection_id.as_deref(),
            Some(saved.id.as_str())
        );
    }
}

#[tokio::test]
async fn collection_regression_allows_deletion_after_recovery_task_settles() {
    for status in [TaskStatus::Completed, TaskStatus::Failed] {
        let ctx = init_test_db().await;
        seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
        let saved = collection::create(&ctx.pool, "rollback", "game-1", "Baseline", true, false)
            .await
            .unwrap();
        collection::runtime::set_active(&ctx.pool, "game-1", Some(&saved.id))
            .await
            .unwrap();
        crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
            &ctx.pool,
            "settled-task",
            "game-1",
            TASK_TYPE_APPLY_COLLECTION,
            Some("target"),
            Some(&saved.id),
            Some(&saved.id),
        )
        .await
        .unwrap();
        assert!(
            crate::modules::workspace::adapters::sqlite::task::compare_and_set_status(
                &ctx.pool,
                "settled-task",
                TaskStatus::Pending,
                status,
            )
            .await
            .unwrap()
        );

        delete_collection(&ctx.pool, &saved.id).await.unwrap();

        assert!(collection::get_by_id(&ctx.pool, &saved.id)
            .await
            .unwrap()
            .is_none());
        assert!(collection::runtime::get(&ctx.pool, "game-1")
            .await
            .unwrap()
            .unwrap()
            .active_collection_id
            .is_none());
    }
}

#[tokio::test]
async fn deleting_active_collection_clears_baseline_without_creating_a_draft() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let active = collection::create(
        &ctx.pool,
        "active-1",
        "game-1",
        "Active Preset",
        true,
        false,
    )
    .await
    .expect("create active collection");
    collection::runtime::set_active(&ctx.pool, "game-1", Some(&active.id))
        .await
        .expect("set active baseline");

    delete_collection(&ctx.pool, &active.id)
        .await
        .expect("delete active collection");

    let runtime = collection::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert!(runtime.active_collection_id.is_none());
    assert!(runtime.draft_collection_id.is_none());
    assert!(collection::get_by_id(&ctx.pool, &active.id)
        .await
        .expect("query deleted collection")
        .is_none());
}

#[tokio::test]
async fn deleting_inactive_collection_preserves_active_baseline_and_draft() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let active = collection::create(
        &ctx.pool,
        "active-1",
        "game-1",
        "Active Preset",
        true,
        false,
    )
    .await
    .expect("create active collection");
    let inactive = collection::create(
        &ctx.pool,
        "inactive-1",
        "game-1",
        "Inactive Preset",
        false,
        false,
    )
    .await
    .expect("create inactive collection");
    let draft = collection::create(&ctx.pool, "draft-1", "game-1", "Last changes", true, true)
        .await
        .expect("create draft");
    let mut tx = ctx.pool.begin().await.expect("begin runtime transaction");
    collection::runtime::set_active_tx(&mut tx, "game-1", Some(&active.id))
        .await
        .expect("set active baseline");
    collection::runtime::set_draft_tx(&mut tx, "game-1", &draft.id, Some(&active.id))
        .await
        .expect("set draft");
    tx.commit().await.expect("commit runtime transaction");

    delete_collection(&ctx.pool, &inactive.id)
        .await
        .expect("delete inactive collection");

    let runtime = collection::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some(active.id.as_str())
    );
    assert_eq!(
        runtime.draft_collection_id.as_deref(),
        Some(draft.id.as_str())
    );
}

#[tokio::test]
async fn deleting_draft_directly_is_rejected() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let draft = collection::create(&ctx.pool, "draft-1", "game-1", "Last changes", true, true)
        .await
        .expect("create draft");

    let error = delete_collection(&ctx.pool, &draft.id)
        .await
        .expect_err("draft deletion must use clear last changes");
    assert!(matches!(error, CollectionError::Validation(_)));
}
