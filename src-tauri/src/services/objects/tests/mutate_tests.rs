use crate::database::object_repo::{CreateObjectInput, UpdateObjectInput};
use crate::services::objects::mutate::{
    create_object_cmd_inner, delete_object, toggle_pin_object, update_object,
};

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_create_object_cmd_inner_success() {
    let pool = setup_test_db().await;

    // Insert a game
    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let input = CreateObjectInput {
        game_id: "g1".to_string(),
        name: "My Object".to_string(),
        folder_path: Some("my_folder".to_string()),
        object_type: "Character".to_string(),
        sub_category: None,
        is_safe: Some(true),
        metadata: None,
    };

    let id = create_object_cmd_inner(&pool, input)
        .await
        .expect("Creation failed");

    // Check if it exists in DB
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_create_object_cmd_inner_conflict() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let input1 = CreateObjectInput {
        game_id: "g1".to_string(),
        name: "Duplicate".to_string(),
        folder_path: None,
        object_type: "Weapon".to_string(),
        sub_category: None,
        is_safe: Some(true),
        metadata: None,
    };

    // First creation should succeed
    create_object_cmd_inner(&pool, input1.clone())
        .await
        .unwrap();

    // Second creation should fail due to unique constraint
    let err = create_object_cmd_inner(&pool, input1).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_toggle_pin_object() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO objects (id, game_id, name, folder_path, object_type) VALUES ('o1', 'g1', 'Obj1', 'path', 'Char')")
        .execute(&pool).await.unwrap();

    toggle_pin_object(&pool, "o1", true).await.unwrap();

    let is_pinned: bool = sqlx::query_scalar("SELECT is_pinned FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(is_pinned);

    toggle_pin_object(&pool, "o1", false).await.unwrap();

    let is_pinned: bool = sqlx::query_scalar("SELECT is_pinned FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!is_pinned);
}

#[tokio::test]
async fn test_update_object() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO objects (id, game_id, name, folder_path, object_type) VALUES ('o1', 'g1', 'Obj1', 'path', 'Char')")
        .execute(&pool).await.unwrap();

    let updates = UpdateObjectInput {
        name: Some("RenamedObj".to_string()),
        object_type: None,
        sub_category: None,
        metadata: None,
        thumbnail_path: None,
        is_safe: None,
        is_auto_sync: None,
        tags: None,
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

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO objects (id, game_id, name, folder_path, object_type) VALUES ('o1', 'g1', 'Obj1', 'path', 'Char')")
        .execute(&pool).await.unwrap();

    delete_object(&pool, "o1").await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_delete_object_with_mods() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO objects (id, game_id, name, folder_path, object_type) VALUES ('o1', 'g1', 'Obj1', 'path', 'Char')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO mods (id, actual_name, folder_path, game_id, object_id, status, is_safe) VALUES ('m1', 'Mod1', 'p', 'g1', 'o1', 'ENABLED', 1)")
        .execute(&pool).await.unwrap();

    // Deletion should fail because mods exist
    let err = delete_object(&pool, "o1").await.unwrap_err();
    assert!(err.to_string().contains("contains mods"));
}
