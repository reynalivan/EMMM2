use super::*;
use crate::domain::task::{RecoveryAction, TaskStatus};
use crate::services::scanner::watcher::WatcherState;
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

async fn setup() -> (sqlx::SqlitePool, ConfigService, WatcherState) {
    let ctx = init_test_db().await;
    // The game must exist before ConfigService is built: it snapshots settings
    // from the DB at construction time.
    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "g1",
            name: "Game One",
            game_type: crate::domain::models::GameType::GIMI,
            path: "C:/Games/One",
            mods_path: Some("C:/Games/One/Mods"),
        },
    )
    .await
    .expect("insert game");
    let config = ConfigService::new_for_test_async(ctx.pool.clone()).await;
    (ctx.pool, config, WatcherState::new())
}

#[tokio::test]
async fn pending_startup_tasks_lists_only_open_work() {
    let (pool, _config, _watcher) = setup().await;
    crate::repo::task_repo::create_task(&pool, "t-open", "g1", "apply_collection", Some("c1"))
        .await
        .expect("create open task");
    crate::repo::task_repo::create_task(&pool, "t-done", "g1", "apply_collection", Some("c1"))
        .await
        .expect("create done task");
    crate::repo::task_repo::update_status(&pool, "t-done", TaskStatus::Completed)
        .await
        .expect("complete task");

    let pending = pending_startup_tasks(&pool).await.expect("list pending");

    let ids: Vec<_> = pending.iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, vec!["t-open"]);
}

#[tokio::test]
async fn missing_task_is_rejected() {
    let (pool, config, watcher) = setup().await;

    let error = resolve_recovery_task(&pool, &config, &watcher, "nope", RecoveryAction::Ignore)
        .await
        .expect_err("unknown task id must fail");

    assert!(format!("{error:?}").contains("nope"));
}

#[tokio::test]
async fn ignore_marks_the_task_failed_without_touching_the_filesystem() {
    let (pool, config, watcher) = setup().await;
    crate::repo::task_repo::create_task(&pool, "t1", "g1", "apply_collection", Some("c1"))
        .await
        .expect("create task");

    resolve_recovery_task(&pool, &config, &watcher, "t1", RecoveryAction::Ignore)
        .await
        .expect("ignore should succeed");

    let task = crate::repo::task_repo::get_task_by_id(&pool, "t1")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Failed);
    // An ignored task must not stay in the startup queue.
    let pending = pending_startup_tasks(&pool).await.expect("list pending");
    assert!(pending.is_empty());
}

#[tokio::test]
async fn retry_rejects_an_unsupported_task_type() {
    let (pool, config, watcher) = setup().await;
    crate::repo::task_repo::create_task(&pool, "t1", "g1", "something_else", None)
        .await
        .expect("create task");

    let error = resolve_recovery_task(&pool, &config, &watcher, "t1", RecoveryAction::Retry)
        .await
        .expect_err("unsupported task type must fail");

    assert!(format!("{error:?}").contains("something_else"));

    // A failed retry must leave the task open so the user can retry again.
    let task = crate::repo::task_repo::get_task_by_id(&pool, "t1")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Pending);
}

#[tokio::test]
async fn retry_requires_a_target_collection() {
    let (pool, config, watcher) = setup().await;
    crate::repo::task_repo::create_task(&pool, "t1", "g1", "apply_collection", None)
        .await
        .expect("create task");

    let error = resolve_recovery_task(&pool, &config, &watcher, "t1", RecoveryAction::Retry)
        .await
        .expect_err("apply_collection without a target must fail");

    assert!(format!("{error:?}").contains("target collection"));
}

#[tokio::test]
async fn legacy_switch_corridor_tasks_are_retired_not_replayed() {
    let (pool, config, watcher) = setup().await;
    crate::repo::task_repo::create_task(&pool, "t1", "g1", "switch_corridor", None)
        .await
        .expect("create task");

    resolve_recovery_task(&pool, &config, &watcher, "t1", RecoveryAction::Retry)
        .await
        .expect("legacy task should be retired quietly");

    let task = crate::repo::task_repo::get_task_by_id(&pool, "t1")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Completed);
}
