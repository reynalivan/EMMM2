use super::*;
use crate::modules::collections::application::collection::tests::{
    create_flat_mod_folder, seed_ainoz_object, seed_game,
};
use crate::modules::games::domain::models::ItemStatus;
use crate::modules::workspace::application::scanner::watcher::WatcherSuppressor;
use crate::test_utils::{init_test_db, insert_test_mod, TestModFixture};
use std::sync::Arc;

struct Fixture {
    db: crate::test_utils::TestContext,
    root: tempfile::TempDir,
    config: ConfigService,
    coordinator: MutationCoordinator,
    _journal: tempfile::TempDir,
}

async fn fixture() -> Fixture {
    let db = init_test_db().await;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().to_string_lossy().to_string();
    seed_game(&db.pool, "game-1", Some(&path)).await;
    seed_ainoz_object(&db.pool, "object-1", "game-1").await;
    for (id, name, safe, enabled) in [
        ("safe", "Safe", true, false),
        ("unsafe", "Private", false, true),
        ("extra", "Extra", true, true),
    ] {
        let path = format!("AINOZ/{}{name}", if enabled { "" } else { "DISABLED " });
        create_flat_mod_folder(root.path(), &path);
        insert_test_mod(
            &db.pool,
            &TestModFixture {
                id,
                game_id: "game-1",
                object_id: Some("object-1"),
                actual_name: name,
                folder_path: &path,
                status: if enabled {
                    ItemStatus::Enabled
                } else {
                    ItemStatus::Disabled
                },
                is_safe: safe,
                object_type: Some("Character"),
                mods_path: Some(&root.path().to_string_lossy()),
            },
        )
        .await
        .unwrap();
    }
    sqlx::query("UPDATE mods SET safety_source = 'manual' WHERE game_id = 'game-1'")
        .execute(&db.pool)
        .await
        .unwrap();
    let config = ConfigService::new_for_test_async(db.pool.clone()).await;
    let journal_dir = tempfile::tempdir().unwrap();
    let journal = Arc::new(
        crate::modules::mutation::journal::OperationJournal::open(
            journal_dir.path().join("journal.json"),
            32,
        )
        .unwrap(),
    );
    let coordinator = MutationCoordinator::with_lock(
        crate::platform::fs::operation_lock::OperationLock::new(),
        journal,
    );
    Fixture {
        db,
        root,
        config,
        coordinator,
        _journal: journal_dir,
    }
}

async fn execute(f: &Fixture, id: &str) -> Result<ApplyResult, AppError> {
    let baseline = task::get_task_by_id(&f.db.pool, id)
        .await?
        .and_then(|task| task.final_active_collection_id);
    execute_safe_mode_transition(
        super::super::ApplyCollectionRequest {
            pool: &f.db.pool,
            game_id: "game-1",
            collection_id: "",
            capture_last_changes: false,
            mods_path: f.root.path().to_path_buf(),
            suppressor: Arc::new(WatcherSuppressor::new(false)),
            ignore_missing: true,
            settings: f.config.get_settings(),
        },
        id,
        baseline,
        &f.config,
        &f.coordinator,
    )
    .await
}

#[path = "safe_mode_state_tests.rs"]
mod state_tests;

#[tokio::test(flavor = "multi_thread")]
async fn safe_mode_preserves_live_edits_and_restores_only_filtered_members() {
    let f = fixture().await;
    let on = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    execute(&f, &on).await.unwrap();
    assert!(f.root.path().join("AINOZ/DISABLED Safe").is_dir());
    assert!(f.root.path().join("AINOZ/DISABLED Private").is_dir());
    assert!(f.root.path().join("AINOZ/Extra").is_dir());
    assert!(f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));

    std::fs::rename(
        f.root.path().join("AINOZ/Extra"),
        f.root.path().join("AINOZ/DISABLED Extra"),
    )
    .unwrap();
    sqlx::query(
        "UPDATE mods SET status = 0, folder_path = 'AINOZ/DISABLED Extra' WHERE id = 'extra'",
    )
    .execute(&f.db.pool)
    .await
    .unwrap();
    let off = prepare_safe_mode_transition(&f.db.pool, "game-1", true)
        .await
        .unwrap();
    execute(&f, &off).await.unwrap();
    assert!(f.root.path().join("AINOZ/Private").is_dir());
    assert!(f.root.path().join("AINOZ/DISABLED Safe").is_dir());
    assert!(f.root.path().join("AINOZ/DISABLED Extra").is_dir());
    assert!(!f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn safe_mode_retry_uses_recorded_intent_after_disk_commit_before_config_sync() {
    let f = fixture().await;
    let id = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    std::fs::rename(
        f.root.path().join("AINOZ/Private"),
        f.root.path().join("AINOZ/DISABLED Private"),
    )
    .unwrap();
    sqlx::query(
        "UPDATE mods SET status = 0, folder_path = 'AINOZ/DISABLED Private' WHERE id = 'unsafe'",
    )
    .execute(&f.db.pool)
    .await
    .unwrap();
    assert!(!f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
    execute(&f, &id).await.unwrap();
    assert!(f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
    assert_eq!(
        task::get_task_by_id(&f.db.pool, &id)
            .await
            .unwrap()
            .unwrap()
            .status,
        TaskStatus::Completed
    );
    assert!(f.root.path().join("AINOZ/DISABLED Private").is_dir());
}

#[tokio::test(flavor = "multi_thread")]
async fn interrupted_safe_mode_rollback_restores_original_flag_and_selection() {
    let f = fixture().await;
    let id = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    std::fs::rename(
        f.root.path().join("AINOZ/Private"),
        f.root.path().join("AINOZ/DISABLED Private"),
    )
    .unwrap();
    sqlx::query(
        "UPDATE mods SET status = 0, folder_path = 'AINOZ/DISABLED Private' WHERE id = 'unsafe'",
    )
    .execute(&f.db.pool)
    .await
    .unwrap();
    storage::safe_mode::request_rollback(&f.db.pool, &id)
        .await
        .unwrap();
    execute(&f, &id).await.unwrap();
    assert!(f.root.path().join("AINOZ/Private").is_dir());
    assert!(!f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
    assert!(storage::safe_mode::get_snapshot(&f.db.pool, "game-1")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn config_failure_keeps_safe_mode_task_recoverable() {
    let f = fixture().await;
    let id = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_settings_insert BEFORE INSERT ON app_settings BEGIN SELECT RAISE(FAIL, 'config write failure'); END")
        .execute(&f.db.pool).await.unwrap();
    assert!(execute(&f, &id).await.is_err());
    assert_eq!(
        task::get_task_by_id(&f.db.pool, &id)
            .await
            .unwrap()
            .unwrap()
            .status,
        TaskStatus::Pending
    );
    assert!(storage::safe_mode::get_intent(&f.db.pool, &id)
        .await
        .unwrap()
        .is_some());
    sqlx::query("DROP TRIGGER fail_settings_insert")
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert!(task::compare_and_set_status(
        &f.db.pool,
        &id,
        TaskStatus::Pending,
        TaskStatus::Running
    )
    .await
    .unwrap());
    execute(&f, &id).await.unwrap();
    assert!(f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn safe_mode_restore_follows_a_semantic_rename() {
    let f = fixture().await;
    let on = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    execute(&f, &on).await.unwrap();
    std::fs::rename(
        f.root.path().join("AINOZ/DISABLED Private"),
        f.root.path().join("AINOZ/DISABLED Renamed"),
    )
    .unwrap();
    let moved = sqlx::query(
        "UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE folder_path_key = ?",
    )
    .bind("AINOZ/DISABLED Renamed")
    .bind(crate::shared::path_key::folder_path_key(
        "AINOZ/Renamed",
        Some(&f.root.path().to_string_lossy()),
    ))
    .bind(crate::shared::path_key::folder_path_key(
        "AINOZ/Private",
        Some(&f.root.path().to_string_lossy()),
    ))
    .execute(&f.db.pool)
    .await
    .unwrap();
    assert_eq!(moved.rows_affected(), 1);
    super::super::handle_mod_moved_or_renamed(
        &f.db.pool,
        "game-1",
        "AINOZ/Private",
        "AINOZ/Renamed",
        None,
    )
    .await
    .unwrap();
    let off = prepare_safe_mode_transition(&f.db.pool, "game-1", true)
        .await
        .unwrap();
    let result = execute(&f, &off).await.unwrap();
    assert!(
        f.root.path().join("AINOZ/Renamed").is_dir(),
        "restore result: {result:?}"
    );
    assert!(!f.root.path().join("AINOZ/Private").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn startup_recovery_reclaims_and_retries_recorded_safe_mode_intent() {
    let f = fixture().await;
    let id = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    assert_eq!(
        task::reclaim_interrupted_apply_tasks(&f.db.pool)
            .await
            .unwrap(),
        1
    );
    let watcher = crate::modules::workspace::application::scanner::watcher::WatcherState::new();
    crate::modules::workspace::application::recovery::resolve_recovery_task(
        crate::modules::workspace::application::recovery::RecoveryTaskRequest {
            pool: &f.db.pool,
            config: &f.config,
            watcher_state: &watcher,
            coordinator: &f.coordinator,
            task_id: &id,
            action: crate::modules::workspace::domain::task::RecoveryAction::Retry,
        },
    )
    .await
    .unwrap();
    assert!(f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
    assert!(f.root.path().join("AINOZ/DISABLED Private").is_dir());
    assert_eq!(
        task::get_task_by_id(&f.db.pool, &id)
            .await
            .unwrap()
            .unwrap()
            .status,
        TaskStatus::Completed
    );
}
