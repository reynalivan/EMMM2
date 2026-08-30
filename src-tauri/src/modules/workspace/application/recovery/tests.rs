use super::*;
use crate::modules::workspace::domain::task::{RecoveryAction, TaskStatus};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
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
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/One",
            mods_path: Some("C:/Games/One/Mods"),
        },
    )
    .await
    .expect("insert game");
    let config = ConfigService::new_for_test_async(ctx.pool.clone()).await;
    (ctx.pool, config, WatcherState::new())
}

async fn setup_apply_game(
    game_id: &str,
) -> (
    sqlx::SqlitePool,
    ConfigService,
    WatcherState,
    tempfile::TempDir,
) {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();
    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: game_id,
            name: "Recovery Game",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: &mods_path,
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert recovery game");
    let config = ConfigService::new_for_test_async(ctx.pool.clone()).await;
    (ctx.pool, config, WatcherState::new(), mods_root)
}

async fn resolve(
    pool: &sqlx::SqlitePool,
    config: &ConfigService,
    watcher: &WatcherState,
    task_id: &str,
    action: RecoveryAction,
) -> Result<(), crate::shared::errors::AppError> {
    resolve_recovery_task(RecoveryTaskRequest {
        pool,
        config,
        watcher_state: watcher,
        task_id,
        action,
    })
    .await
}

#[tokio::test]
async fn pending_startup_tasks_lists_only_open_work() {
    let (pool, _config, _watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "t-done", "g1", "apply_collection", Some("c1"))
        .await
        .expect("create done task");
    crate::modules::workspace::adapters::sqlite::task::update_status(&pool, "t-done", TaskStatus::Completed)
        .await
        .expect("complete task");
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "t-open", "g1", "apply_collection", Some("c1"))
        .await
        .expect("create open task");

    let pending = crate::modules::workspace::adapters::sqlite::task::get_all_pending_tasks_global(&pool)
        .await
        .expect("list pending");

    let ids: Vec<_> = pending.iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, vec!["t-open"]);
}

#[tokio::test]
async fn missing_task_is_rejected() {
    let (pool, config, watcher) = setup().await;

    let error = resolve(&pool, &config, &watcher, "nope", RecoveryAction::Ignore)
        .await
        .expect_err("unknown task id must fail");

    assert!(format!("{error:?}").contains("nope"));
}

#[tokio::test]
async fn ignore_marks_the_task_failed_without_touching_the_filesystem() {
    let (pool, config, watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "t1", "g1", "apply_collection", Some("c1"))
        .await
        .expect("create task");

    resolve(&pool, &config, &watcher, "t1", RecoveryAction::Ignore)
        .await
        .expect("ignore should succeed");

    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "t1")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Failed);
    // An ignored task must not stay in the startup queue.
    let pending = crate::modules::workspace::adapters::sqlite::task::get_all_pending_tasks_global(&pool)
        .await
        .expect("list pending");
    assert!(pending.is_empty());
}

#[tokio::test]
async fn retry_rejects_an_unsupported_task_type() {
    let (pool, config, watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "t1", "g1", "something_else", None)
        .await
        .expect("create task");

    let error = resolve(&pool, &config, &watcher, "t1", RecoveryAction::Retry)
        .await
        .expect_err("unsupported task type must fail");

    assert!(format!("{error:?}").contains("something_else"));

    // A failed retry must leave the task open so the user can retry again.
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "t1")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Pending);
}

#[tokio::test]
async fn retry_requires_a_target_collection() {
    let (pool, config, watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "t1", "g1", "apply_collection", None)
        .await
        .expect("create task");

    let error = resolve(&pool, &config, &watcher, "t1", RecoveryAction::Retry)
        .await
        .expect_err("apply_collection without a target must fail");

    assert!(format!("{error:?}").contains("target collection"));
}

#[tokio::test]
async fn successful_retry_promotes_the_applied_collection_to_active_baseline() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();
    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "g-retry",
            name: "Retry Game",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: &mods_path,
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert game");
    let target = crate::modules::collections::adapters::sqlite::create(
        &ctx.pool,
        "target-collection",
        "g-retry",
        "Target",
        true,
        false,
    )
    .await
    .expect("create collection");
    let empty_state = crate::modules::workspace::application::projected_state::empty_projected_state();
    crate::modules::collections::application::collection::persist_projected_state(
        &ctx.pool,
        &target.id,
        &[],
        &[],
        &empty_state,
    )
    .await
    .expect("persist collection state");
    crate::modules::workspace::adapters::sqlite::task::create_task(
        &ctx.pool,
        "retry-task",
        "g-retry",
        "apply_collection",
        Some(&target.id),
    )
    .await
    .expect("create retry task");

    let config = ConfigService::new_for_test_async(ctx.pool.clone()).await;
    let watcher = WatcherState::new();
    resolve(
        &ctx.pool,
        &config,
        &watcher,
        "retry-task",
        RecoveryAction::Retry,
    )
    .await
    .expect("retry collection apply");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "g-retry")
        .await
        .expect("load runtime")
        .expect("runtime state");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some(target.id.as_str())
    );
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&ctx.pool, "retry-task")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Completed);
    let task_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE game_id = 'g-retry' AND task_type = 'apply_collection'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("count apply tasks");
    assert_eq!(task_count, 1, "retry must reuse its original task");
}

#[tokio::test]
async fn retrying_a_restore_task_uses_its_stored_final_baseline_not_the_draft_target() {
    let (pool, config, watcher, _mods_root) = setup_apply_game("g-restore-retry").await;
    create_empty_collection_for_game(
        &pool,
        "g-restore-retry",
        "restore-baseline",
        "Baseline",
        false,
    )
    .await;
    create_empty_collection_for_game(
        &pool,
        "g-restore-retry",
        "restore-draft",
        "Last changes",
        true,
    )
    .await;
    crate::modules::workspace::adapters::sqlite::task::create_task_with_full_intent(
        &pool,
        "restore-retry-task",
        "g-restore-retry",
        "apply_collection",
        Some("restore-draft"),
        None,
        None,
        Some("restore-baseline"),
    )
    .await
    .expect("create interrupted restore task");

    resolve(
        &pool,
        &config,
        &watcher,
        "restore-retry-task",
        RecoveryAction::Retry,
    )
    .await
    .expect("retry restore task");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&pool, "g-restore-retry")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some("restore-baseline")
    );
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "restore-retry-task")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(
        task.final_active_collection_id.as_deref(),
        Some("restore-baseline")
    );
    assert_eq!(task.status, TaskStatus::Completed);
}

#[tokio::test]
async fn retry_finalization_failure_keeps_original_task_and_active_baseline() {
    let (pool, config, watcher, _mods_root) = setup_apply_game("g-retry-atomic").await;
    for (id, name) in [("baseline", "Baseline"), ("target", "Target")] {
        create_empty_collection_for_game(&pool, "g-retry-atomic", id, name, false).await;
    }
    crate::modules::collections::adapters::sqlite::runtime::set_active(&pool, "g-retry-atomic", Some("baseline"))
        .await
        .expect("set baseline");
    crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
        &pool,
        "retry-original",
        "g-retry-atomic",
        "apply_collection",
        Some("target"),
        Some("baseline"),
        Some("baseline"),
    )
    .await
    .expect("create recovery task");
    sqlx::query(
        r#"CREATE TRIGGER fail_retry_task_finalize
           BEFORE UPDATE OF status ON tasks
           WHEN OLD.id = 'retry-original' AND NEW.status = 'COMPLETED'
           BEGIN SELECT RAISE(ABORT, 'injected retry task failure'); END"#,
    )
    .execute(&pool)
    .await
    .expect("install task failpoint");

    resolve(
        &pool,
        &config,
        &watcher,
        "retry-original",
        RecoveryAction::Retry,
    )
    .await
    .expect_err("task finalization failure must fail recovery");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&pool, "g-retry-atomic")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(runtime.active_collection_id.as_deref(), Some("baseline"));
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "retry-original")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Pending);
    let task_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE game_id = 'g-retry-atomic' AND task_type = 'apply_collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("count tasks");
    assert_eq!(task_count, 1, "recovery must reuse the original task");
}

#[tokio::test]
async fn rollback_finalization_failure_keeps_pre_recovery_active_pointer() {
    let (pool, config, watcher, _mods_root) = setup_apply_game("g-rollback-atomic").await;
    for (id, name, is_draft) in [
        ("baseline", "Baseline", false),
        ("target", "Target", false),
        ("draft", "Last changes", true),
    ] {
        create_empty_collection_for_game(&pool, "g-rollback-atomic", id, name, is_draft).await;
    }
    crate::modules::collections::adapters::sqlite::runtime::set_active(&pool, "g-rollback-atomic", Some("target"))
        .await
        .expect("set target active");
    crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
        &pool,
        "rollback-original",
        "g-rollback-atomic",
        "apply_collection",
        Some("target"),
        Some("draft"),
        Some("baseline"),
    )
    .await
    .expect("create recovery task");
    sqlx::query(
        r#"CREATE TRIGGER fail_rollback_task_finalize
           BEFORE UPDATE OF status ON tasks
           WHEN OLD.id = 'rollback-original' AND NEW.status = 'COMPLETED'
           BEGIN SELECT RAISE(ABORT, 'injected rollback task failure'); END"#,
    )
    .execute(&pool)
    .await
    .expect("install task failpoint");

    resolve(
        &pool,
        &config,
        &watcher,
        "rollback-original",
        RecoveryAction::Rollback,
    )
    .await
    .expect_err("task finalization failure must fail recovery");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&pool, "g-rollback-atomic")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(runtime.active_collection_id.as_deref(), Some("target"));
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "rollback-original")
        .await
        .expect("load task")
        .expect("task exists");
    assert_eq!(task.status, TaskStatus::Pending);
    let task_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE game_id = 'g-rollback-atomic' AND task_type = 'apply_collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("count tasks");
    assert_eq!(task_count, 1, "recovery must reuse the original task");
}

#[tokio::test]
async fn concurrent_ignore_actions_have_exactly_one_winner() {
    let (pool, config, watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "claim-once", "g1", "apply_collection", None)
        .await
        .expect("create task");

    let (first, second) = tokio::join!(
        resolve(
            &pool,
            &config,
            &watcher,
            "claim-once",
            RecoveryAction::Ignore,
        ),
        resolve(
            &pool,
            &config,
            &watcher,
            "claim-once",
            RecoveryAction::Ignore,
        )
    );

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
}

#[tokio::test]
async fn running_or_settled_tasks_reject_additional_recovery_actions() {
    let (pool, config, watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "claimed", "g1", "apply_collection", None)
        .await
        .expect("create claimed task");
    sqlx::query("UPDATE tasks SET status = 'RUNNING' WHERE id = 'claimed'")
        .execute(&pool)
        .await
        .expect("mark task running");
    resolve(&pool, &config, &watcher, "claimed", RecoveryAction::Ignore)
        .await
        .expect_err("a claimed task must reject Ignore");
    resolve(&pool, &config, &watcher, "claimed", RecoveryAction::Retry)
        .await
        .expect_err("a claimed task must reject an opposing retry");
    crate::modules::workspace::adapters::sqlite::task::update_status(&pool, "claimed", TaskStatus::Completed)
        .await
        .expect("settle claimed task");

    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "settled", "g1", "apply_collection", None)
        .await
        .expect("create settled task");
    crate::modules::workspace::adapters::sqlite::task::update_status(&pool, "settled", TaskStatus::Completed)
        .await
        .expect("settle task");
    resolve(&pool, &config, &watcher, "settled", RecoveryAction::Ignore)
        .await
        .expect_err("a settled task must reject Ignore");
    resolve(
        &pool,
        &config,
        &watcher,
        "settled",
        RecoveryAction::Rollback,
    )
    .await
    .expect_err("a settled task must reject an opposing rollback");
}

#[tokio::test]
async fn startup_check_is_read_only_and_backend_boot_reclaims_crashed_recovery() {
    let (pool, config, watcher) = setup().await;
    crate::modules::workspace::adapters::sqlite::task::create_task(&pool, "crashed", "g1", "apply_collection", None)
        .await
        .expect("create task");
    assert!(crate::modules::workspace::adapters::sqlite::task::compare_and_set_status(
        &pool,
        "crashed",
        TaskStatus::Pending,
        TaskStatus::Running,
    )
    .await
    .expect("claim task"));
    resolve(&pool, &config, &watcher, "crashed", RecoveryAction::Ignore)
        .await
        .expect_err("same-session action must reject a running task");

    let startup_tasks = get_startup_recovery_tasks(&pool)
        .await
        .expect("load startup recovery tasks");
    assert_eq!(startup_tasks.len(), 1);
    assert_eq!(startup_tasks[0].status, TaskStatus::Running);
    resolve(&pool, &config, &watcher, "crashed", RecoveryAction::Ignore)
        .await
        .expect_err("repeatable startup check must not revoke a live claim");

    let reclaimed_count = crate::modules::workspace::adapters::sqlite::task::reclaim_interrupted_apply_tasks(&pool)
        .await
        .expect("backend boot should reclaim interrupted tasks");
    assert_eq!(reclaimed_count, 1);
    let reclaimed = get_startup_recovery_tasks(&pool)
        .await
        .expect("list reclaimed startup task");
    assert_eq!(reclaimed[0].status, TaskStatus::Pending);

    resolve(&pool, &config, &watcher, "crashed", RecoveryAction::Ignore)
        .await
        .expect("reclaimed task should be actionable");
}

#[tokio::test]
async fn ignore_cannot_settle_a_normal_apply_waiting_to_mutate_disk() {
    let (pool, config, watcher, mods_root) = setup_apply_game("g-apply-claim").await;
    create_empty_collection_for_game(&pool, "g-apply-claim", "target", "Target", false).await;
    let entered = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let release = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    crate::modules::collections::application::collection::set_apply_execution_barrier(Some(
        crate::modules::collections::application::collection::ApplyExecutionBarrier {
            game_id: "g-apply-claim".to_string(),
            entered: entered.clone(),
            release: release.clone(),
        },
    ));

    let apply_pool = pool.clone();
    let apply_suppressor = watcher.suppressor.clone();
    let apply_settings = config.get_settings();
    let apply_path = mods_root.path().to_path_buf();
    let apply = tokio::spawn(async move {
        crate::modules::collections::application::collection::apply_collection(
            crate::modules::collections::application::collection::ApplyCollectionRequest {
                pool: &apply_pool,
                game_id: "g-apply-claim",
                collection_id: "target",
                capture_last_changes: true,
                mods_path: apply_path,
                suppressor: apply_suppressor,
                ignore_missing: false,
                settings: apply_settings,
            },
        )
        .await
    });

    entered.wait().await;
    let open_task: (String, String) = sqlx::query_as(
        "SELECT id, status FROM tasks WHERE game_id = 'g-apply-claim' AND task_type = 'apply_collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("load normal apply task");
    let ignore = resolve(
        &pool,
        &config,
        &watcher,
        &open_task.0,
        RecoveryAction::Ignore,
    )
    .await;
    release.wait().await;
    crate::modules::collections::application::collection::set_apply_execution_barrier(None);
    let apply_result = apply.await.expect("join normal apply");

    assert_eq!(open_task.1, TaskStatus::Running.as_str());
    assert!(
        ignore.is_err(),
        "Ignore must not settle a live normal apply"
    );
    assert!(apply_result.is_ok(), "normal apply should retain its claim");
}

async fn create_empty_collection(pool: &sqlx::SqlitePool, id: &str, name: &str, is_draft: bool) {
    create_empty_collection_for_game(pool, "g1", id, name, is_draft).await;
}

async fn create_empty_collection_for_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    id: &str,
    name: &str,
    is_draft: bool,
) {
    crate::modules::collections::adapters::sqlite::create(pool, id, game_id, name, true, is_draft)
        .await
        .expect("create collection");
    crate::modules::collections::application::collection::persist_projected_state(
        pool,
        id,
        &[],
        &[],
        &crate::modules::workspace::application::projected_state::empty_projected_state(),
    )
    .await
    .expect("persist empty state");
}

#[tokio::test]
async fn rollback_to_draft_without_baseline_stays_unsaved() {
    let (pool, _config, _watcher) = setup().await;
    create_empty_collection(&pool, "draft", "Last changes", true).await;
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut pool.acquire().await.unwrap(),
        "g1",
        "draft",
        None,
    )
    .await
    .unwrap();
    crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
        &pool,
        "rollback",
        "g1",
        "apply_collection",
        Some("target"),
        Some("draft"),
        None,
    )
    .await
    .unwrap();
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "rollback")
        .await
        .unwrap()
        .unwrap();

    let rollback = resolve_rollback_target(&pool, &task)
        .await
        .expect("resolve draft rollback");

    assert_eq!(rollback.collection_id, "draft");
    assert_eq!(rollback.active_baseline_id, None);
}

#[tokio::test]
async fn rollback_to_draft_with_deleted_baseline_stays_unsaved() {
    let (pool, _config, _watcher) = setup().await;
    create_empty_collection(&pool, "baseline", "Baseline", false).await;
    create_empty_collection(&pool, "draft", "Last changes", true).await;
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut pool.acquire().await.unwrap(),
        "g1",
        "draft",
        Some("baseline"),
    )
    .await
    .unwrap();
    crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
        &pool,
        "rollback",
        "g1",
        "apply_collection",
        Some("target"),
        Some("draft"),
        Some("baseline"),
    )
    .await
    .unwrap();
    sqlx::query("DELETE FROM collections WHERE id = 'baseline'")
        .execute(&pool)
        .await
        .unwrap();
    let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(&pool, "rollback")
        .await
        .unwrap()
        .unwrap();

    let rollback = resolve_rollback_target(&pool, &task)
        .await
        .expect("resolve draft rollback");

    assert_eq!(rollback.collection_id, "draft");
    assert_eq!(rollback.active_baseline_id, None);
}

mod characterization;
