use super::*;
use crate::database::game_repo::{upsert_game, GameRow};
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_object_crud() {
    let pool = setup_pool().await;

    // Insert game
    let game = GameRow {
        id: "g1".into(),
        name: "Game 1".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game1".into(),
        mod_path: None,
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // Create object
    create_object(
        &pool,
        "obj1",
        "g1",
        "Obj Name",
        "Mods/Obj1",
        "Character",
        None,
        false,
        "{}",
    )
    .await
    .unwrap();

    // Test get name
    let name = get_object_name_by_id(&pool, "obj1").await.unwrap();
    assert_eq!(name.as_deref(), Some("Obj Name"));

    // Test get path
    let path = get_folder_path(&pool, "obj1").await.unwrap();
    assert_eq!(path.as_deref(), Some("Mods/Obj1"));

    // Update object
    let updates = UpdateObjectInput {
        name: Some("New Obj Name".into()),
        object_type: None,
        sub_category: None,
        is_safe: Some(true),
        metadata: None,
        thumbnail_path: None,
        is_auto_sync: None,
        tags: None,
    };
    update_object(&pool, "obj1", &updates).await.unwrap();

    let name2 = get_object_name_by_id(&pool, "obj1").await.unwrap();
    assert_eq!(name2.as_deref(), Some("New Obj Name"));

    // Get count
    let counts = get_category_counts(&pool, "g1", false).await.unwrap();
    assert!(!counts.is_empty());
    assert_eq!(counts[0].object_type, "Character");
    assert_eq!(counts[0].count, 1);

    // Test filter
    let filter = ObjectFilter {
        game_id: "g1".to_string(),
        search_query: None,
        object_type: Some("Character".to_string()),
        safe_mode: true, // we updated it to true!
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };

    let objects = get_filtered_objects(&pool, &filter).await.unwrap();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].name, "New Obj Name");

    // Delete
    delete_object(&pool, "obj1").await.unwrap();
    let name_del = get_object_name_by_id(&pool, "obj1").await.unwrap();
    assert!(name_del.is_none());
}
