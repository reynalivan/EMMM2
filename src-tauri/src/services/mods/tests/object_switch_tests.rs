use super::toggle_object_root_service;
use crate::domain::models::{GameType, ItemStatus};
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;
use crate::test_utils::{insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture};

#[tokio::test]
async fn object_switch_reports_rewrite_when_db_disabled_but_disk_enabled() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mods_path = temp_dir.path().join("mods");
    let enabled_path = mods_path.join("Alice");
    std::fs::create_dir_all(&enabled_path).expect("object folder should exist");

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_object_switch_rewrite",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_object_switch_rewrite",
            mods_path: Some(mods_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o_object_switch_rewrite",
            game_id: "g_object_switch_rewrite",
            name: "Alice",
            folder_path: "DISABLED Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    let config = crate::services::config::ConfigService::new_for_test_async(pool.clone()).await;
    let watcher_state = WatcherState::new();
    let op_lock = OperationLock::new();

    let outcome = toggle_object_root_service(
        &config,
        &pool,
        &watcher_state,
        &op_lock,
        "g_object_switch_rewrite",
        "o_object_switch_rewrite",
        true,
    )
    .await
    .unwrap();

    assert!(outcome.original_path.ends_with("DISABLED Alice"));
    assert_eq!(outcome.next_path, enabled_path.to_string_lossy());

    let row: (String, i64) = sqlx::query_as("SELECT folder_path, status FROM objects WHERE id = ?")
        .bind("o_object_switch_rewrite")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, "Alice");
    assert_eq!(row.1, ItemStatus::Enabled as i64);
}
