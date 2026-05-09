use crate::repo::object_repo::{CreateObjectInput, UpdateObjectInput};
use crate::services::objects::mutate::{
    create_object_cmd_inner, delete_object, toggle_pin_object, update_object,
};

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_create_object_cmd_inner_success() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let mods_path = tmp.path().join("Mods");
    std::fs::create_dir(&mods_path).unwrap();
    let mods_path_str = mods_path.to_string_lossy().to_string();

    // Insert a game
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some(&mods_path_str),
        },
    )
    .await
    .unwrap();

    let input = CreateObjectInput {
        game_id: "g1".to_string(),
        name: "My Object".to_string(),
        folder_path: Some("my_folder".to_string()),
        object_type: "Character".to_string(),
        sub_category: None,
        status: None,
        metadata: None,
        thumbnail_url: None,
        hash_db: None,
        custom_skins: None,
    };

    let id = create_object_cmd_inner(&pool, None, input)
        .await
        .expect("Creation failed");

    // Check if it exists in DB
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert!(mods_path.join("my_folder").is_dir());
}

#[tokio::test]
async fn test_create_object_cmd_inner_conflict() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let mods_path = tmp.path().join("Mods");
    std::fs::create_dir(&mods_path).unwrap();
    let mods_path_str = mods_path.to_string_lossy().to_string();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some(&mods_path_str),
        },
    )
    .await
    .unwrap();

    let input1 = CreateObjectInput {
        game_id: "g1".to_string(),
        name: "Duplicate".to_string(),
        folder_path: None,
        object_type: "Weapon".to_string(),
        sub_category: None,
        status: None,
        metadata: None,
        thumbnail_url: None,
        hash_db: None,
        custom_skins: None,
    };

    // First creation should succeed
    create_object_cmd_inner(&pool, None, input1.clone())
        .await
        .unwrap();

    // Second creation should fail due to unique constraint
    let err = create_object_cmd_inner(&pool, None, input1)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_create_object_cmd_inner_does_not_leave_db_row_when_folder_creation_fails() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let mods_path_file = tmp.path().join("ModsAsFile");
    std::fs::write(&mods_path_file, "not a directory").unwrap();
    let mods_path_str = mods_path_file.to_string_lossy().to_string();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g_fs_fail",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some(&mods_path_str),
        },
    )
    .await
    .unwrap();

    let input = CreateObjectInput {
        game_id: "g_fs_fail".to_string(),
        name: "Broken Object".to_string(),
        folder_path: Some("broken_folder".to_string()),
        object_type: "Character".to_string(),
        sub_category: None,
        status: None,
        metadata: None,
        thumbnail_url: None,
        hash_db: None,
        custom_skins: None,
    };

    let err = create_object_cmd_inner(&pool, None, input)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Failed to create object folder"));

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g_fs_fail")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_toggle_pin_object() {
    let pool = setup_test_db().await;

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Obj1",
            folder_path: Some("path"),
            object_type: "Char",
        },
    )
    .await
    .unwrap();

    toggle_pin_object(&pool, "o1", true).await.unwrap();

    let is_pinned: i32 = sqlx::query_scalar("SELECT is_pinned FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(is_pinned, 1);

    toggle_pin_object(&pool, "o1", false).await.unwrap();

    let is_pinned: i32 = sqlx::query_scalar("SELECT is_pinned FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(is_pinned, 0);
}

#[tokio::test]
async fn test_update_object() {
    let pool = setup_test_db().await;

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Obj1",
            folder_path: Some("path"),
            object_type: "Char",
        },
    )
    .await
    .unwrap();

    let updates = UpdateObjectInput {
        name: Some("RenamedObj".to_string()),
        object_type: None,
        sub_category: None,
        status: None,
        metadata: None,
        thumbnail_path: None,
        is_auto_sync: None,
        is_pinned: None,
        tags: None,
        hash_db: None,
        custom_skins: None,
    };

    update_object(&pool, "o1", &updates).await.unwrap();

    let name: String = sqlx::query_scalar("SELECT name FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "RenamedObj");
}

#[tokio::test]
async fn test_delete_object_empty() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let trash_dir = tmp.path().join("trash");
    std::fs::create_dir(&trash_dir).unwrap();
    let watcher_state = crate::services::scanner::watcher::WatcherState::default();
    let op_lock = crate::services::fs_utils::operation_lock::OperationLock::new();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Obj1",
            folder_path: Some("path"),
            object_type: "Char",
        },
    )
    .await
    .unwrap();

    delete_object(&pool, "o1", false, &trash_dir, &watcher_state, &op_lock)
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_delete_object_cascade_mods() {
    let pool = setup_test_db().await;
    let tmp = tempfile::TempDir::new().unwrap();
    let trash_dir = tmp.path().join("trash");
    std::fs::create_dir(&trash_dir).unwrap();
    let watcher_state = crate::services::scanner::watcher::WatcherState::default();
    let op_lock = crate::services::fs_utils::operation_lock::OperationLock::new();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Obj1",
            folder_path: Some("path"),
            object_type: "Char",
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "m1",
            game_id: "g1",
            object_id: Some("o1"),
            actual_name: "Mod1",
            folder_path: "p1",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Char"),
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "m2",
            game_id: "g1",
            object_id: Some("o1"),
            actual_name: "Mod2",
            folder_path: "p2",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Char"),
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    // Deletion should cascade — remove mods + object
    delete_object(&pool, "o1", true, &trash_dir, &watcher_state, &op_lock)
        .await
        .unwrap();

    let obj_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(obj_count, 0, "Object should be deleted");

    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(mod_count, 0, "Child mods should be cascade-deleted");
}
