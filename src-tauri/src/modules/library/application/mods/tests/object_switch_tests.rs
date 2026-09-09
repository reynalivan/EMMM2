use super::{prepare_object_root_switch, toggle_object_root_service};
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::operation_lock::OperationLock;
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

    // Pin the single-writer contract: seed status=Disabled so a service that
    // still wrote status (it must not) would flip it to Enabled below.
    sqlx::query("UPDATE objects SET status = 0 WHERE id = ?")
        .bind("o_object_switch_rewrite")
        .execute(&pool)
        .await
        .unwrap();

    let watcher_state = WatcherState::new();
    let op_lock = OperationLock::new();
    let op_guard = op_lock.acquire().await.unwrap();

    let outcome = toggle_object_root_service(
        &pool,
        &watcher_state,
        &op_guard,
        "g_object_switch_rewrite",
        "o_object_switch_rewrite",
        true,
    )
    .await
    .unwrap();

    assert!(outcome.original_path.ends_with("DISABLED Alice"));
    assert_eq!(outcome.next_path, enabled_path.to_string_lossy());

    // Single-writer contract: the service never writes `status` — that column
    // converges via the caller's scoped disk reconcile. Only the resolve-time
    // path heal touched `folder_path` here.
    let row: (String, i64) = sqlx::query_as("SELECT folder_path, status FROM objects WHERE id = ?")
        .bind("o_object_switch_rewrite")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, "Alice");
    assert_eq!(row.1, ItemStatus::Disabled as i64);
}

#[tokio::test]
async fn preparing_object_switch_does_not_heal_the_stored_path() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mods_path = temp_dir.path().join("mods");
    std::fs::create_dir_all(mods_path.join("Alice")).expect("object folder should exist");

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_object_switch_prepare",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_object_switch_prepare",
            mods_path: Some(mods_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o_object_switch_prepare",
            game_id: "g_object_switch_prepare",
            name: "Alice",
            folder_path: "DISABLED Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    prepare_object_root_switch(
        &pool,
        "g_object_switch_prepare",
        "o_object_switch_prepare",
        true,
    )
    .await
    .unwrap();

    let stored_path: String = sqlx::query_scalar("SELECT folder_path FROM objects WHERE id = ?")
        .bind("o_object_switch_prepare")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored_path, "DISABLED Alice");
}
