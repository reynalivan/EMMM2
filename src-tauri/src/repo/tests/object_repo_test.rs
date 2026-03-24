use super::*;
use crate::repo::game_repo::{upsert_game, GameRow};
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
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\Game1".into(),
        mods_path: Some("C:\\Mods".into()),
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
        None,
        "{}",
        None,
        None,
        None,
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
        status: None,
        metadata: None,
        hash_db: None,
        custom_skins: None,
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

#[tokio::test]
async fn ensure_object_exists_matches_unicode_names_without_duplicate_rows() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "g_unicode".into(),
        name: "Game Unicode".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\GameUnicode".into(),
        mods_path: Some("C:\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    create_object(
        &pool,
        "obj_unicode",
        "g_unicode",
        "Preset_日本語",
        "한글MOD",
        "Character",
        None,
        None,
        "{}",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let mut new_objects_count = 0;
    let object_id = ensure_object_exists(
        &mut tx,
        "g_unicode",
        "한글mod",
        "preset_日本語",
        "Character",
        None,
        "[]",
        "{}",
        None,
        None,
        &mut new_objects_count,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(object_id, "obj_unicode");
    assert_eq!(new_objects_count, 0);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g_unicode")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn delete_object_and_mods_by_folder_matches_unicode_prefix_with_ascii_case_variants() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "g_delete".into(),
        name: "Game Delete".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\GameDelete".into(),
        mods_path: Some("C:\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    create_object(
        &pool,
        "obj_delete",
        "g_delete",
        "한국Character",
        "한국Character",
        "Character",
        None,
        None,
        "{}",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    crate::repo::mod_repo::insert_new_mod(
        &pool,
        "child_delete",
        "g_delete",
        "obj_delete",
        "日本語Mod",
        "한국Character/日本語Mod",
        Some("C:\\Mods"),
        crate::database::models::ItemStatus::Enabled,
    )
    .await
    .unwrap();

    let deleted = {
        let mut tx = pool.acquire().await.unwrap();
        delete_object_and_mods_by_folder(&mut tx, "g_delete", "한국character")
            .await
            .unwrap()
    };

    assert_eq!(deleted, 1);

    let remaining_objects: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
            .bind("g_delete")
            .fetch_one(&pool)
            .await
            .unwrap();
    let remaining_mods: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
        .bind("g_delete")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(remaining_objects, 0);
    assert_eq!(remaining_mods, 0);
}
