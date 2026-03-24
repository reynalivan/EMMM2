use super::*;
use crate::repo::game_repo::{upsert_game, GameRow};
use crate::repo::object_repo::create_object;
use crate::services::path_key::folder_path_key;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_mod_repo_crud() {
    let pool = setup_pool().await;

    // Insert game
    let game = GameRow {
        id: "g1".into(),
        name: "Game 1".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\Game1".into(),
        mods_path: Some("C:".into()),
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
        "Test Object",
        "Test Object",
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

    // Insert Mod
    insert_new_mod(
        &pool,
        "mod1",
        "g1",
        "obj1",
        "My Mod",
        "Mods/Obj1/Mod1",
        Some("C:"),
        crate::database::models::ItemStatus::Enabled,
        true,
        "manual",
    )
    .await
    .unwrap();

    // Set mod object id manually
    // (Already set in insert_new_mod)

    // Test get_object_id_by_path
    // We pass "Mods/Obj1/Mod1" as the relative path.
    // And "C:" as the mods_path to get a consistent key.
    let obj_id = get_object_id_by_path(&pool, "g1", "Mods/Obj1/Mod1")
        .await
        .unwrap();
    assert_eq!(obj_id.as_deref(), Some("obj1"));

    // Test get_mod_by_object_id
    let mod_info = get_mod_by_object_id(&pool, "obj1").await.unwrap();
    assert!(mod_info.is_some());
    assert_eq!(mod_info.unwrap().1, "Mods/Obj1/Mod1");

    // Test favorite & pin
    set_favorite_by_path(&pool, "g1", "Mods/Obj1/Mod1", true)
        .await
        .unwrap();
    set_pinned_by_path(&pool, "g1", "Mods/Obj1/Mod1", true)
        .await
        .unwrap();

    // Update path and status
    update_mod_path_status_and_reason(
        &pool,
        "g1",
        "Mods/Obj1/Mod1",
        "Mods/Obj1/Mod2",
        crate::database::models::ItemStatus::Disabled,
        None,
    )
    .await
    .unwrap();

    // Test delete by id
    delete_mod_by_id(&pool, "mod1").await.unwrap();
    let mod_info2 = get_mod_by_object_id(&pool, "obj1").await.unwrap();
    assert!(mod_info2.is_none());
}

#[tokio::test]
async fn update_child_paths_matches_unicode_prefix_with_ascii_case_variants() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "g_unicode".into(),
        name: "Game Unicode".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\GameUnicode".into(),
        mods_path: Some("C:".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // Create object for FK
    create_object(
        &pool,
        "obj_unicode",
        "g_unicode",
        "Obj",
        "한국Renamed",
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

    insert_new_mod(
        &pool,
        "child_mod",
        "g_unicode",
        "obj_unicode", // dummy object id
        "日本語Variant",
        "한국Character/日本語Root/VariantA",
        Some("C:"),
        crate::database::models::ItemStatus::Enabled,
        true,
        "manual",
    )
    .await
    .unwrap();

    update_child_paths(
        &pool,
        "g_unicode",
        "한국character\\",
        "한국Renamed\\",
        Some("C:"),
    )
    .await
    .unwrap();

    let updated_path: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = ?")
        .bind("child_mod")
        .fetch_one(&pool)
        .await
        .unwrap();
    let updated_key: String = sqlx::query_scalar("SELECT folder_path_key FROM mods WHERE id = ?")
        .bind("child_mod")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(updated_path, "한국Renamed/日本語Root/VariantA");
    assert_eq!(
        updated_key,
        folder_path_key("한국Renamed/日本語Root/VariantA", Some("C:"))
    );
}

#[tokio::test]
async fn update_status_for_object_matches_unicode_object_folder_with_ascii_case_variants() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "g_status".into(),
        name: "Game Status".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\GameStatus".into(),
        mods_path: Some("C:".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // Create object for FK
    create_object(
        &pool,
        "obj_status",
        "g_status",
        "Obj",
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

    insert_new_mod(
        &pool,
        "status_mod",
        "g_status",
        "obj_status", // dummy object id
        "中文Mod",
        "한국Character/中文Mod",
        Some("C:"),
        crate::database::models::ItemStatus::Enabled,
        true,
        "manual",
    )
    .await
    .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    update_status_for_object(
        &mut conn,
        "g_status",
        "한국character",
        crate::database::models::ItemStatus::Disabled,
    )
    .await
    .unwrap();
    drop(conn);

    let status: crate::database::models::ItemStatus =
        sqlx::query_scalar("SELECT status FROM mods WHERE id = ?")
            .bind("status_mod")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(status, crate::database::models::ItemStatus::Disabled);
}

#[tokio::test]
async fn test_repo_mod_status_consistency() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "g_consist".into(),
        name: "Game Consist".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\GameConsist".into(),
        mods_path: Some("C:".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // Create object
    create_object(
        &pool,
        "obj_consist",
        "g_consist",
        "Obj",
        "Obj",
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

    // 1. Insert Enabled mod
    insert_new_mod(
        &pool,
        "mod_enabled",
        "g_consist",
        "obj_consist",
        "Enabled Mod",
        "Mods/EnabledMod",
        Some("C:"),
        crate::database::models::ItemStatus::Enabled,
        true,
        "manual",
    )
    .await
    .unwrap();

    // 2. Insert Disabled mod
    insert_new_mod(
        &pool,
        "mod_disabled",
        "g_consist",
        "obj_consist",
        "Disabled Mod",
        "Mods/DisabledMod",
        Some("C:"),
        crate::database::models::ItemStatus::Disabled,
        true,
        "manual",
    )
    .await
    .unwrap();

    // 3. Verify raw integer values in DB
    let enabled_raw: i64 = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'mod_enabled'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let disabled_raw: i64 = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'mod_disabled'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(enabled_raw, 1, "Enabled should be 1 in DB");
    assert_eq!(disabled_raw, 0, "Disabled should be 0 in DB");

    // 4. Verify our manual SQL with status = 1 works
    let found_enabled: bool =
        sqlx::query_scalar::<sqlx::Sqlite, i32>("SELECT 1 FROM mods WHERE id = ? AND status = 1")
            .bind("mod_enabled")
            .fetch_optional(&pool)
            .await
            .unwrap()
            .is_some();
    assert!(
        found_enabled,
        "Manual SQL 'status = 1' should find enabled mod"
    );

    // 5. Verify our manual SQL with status = 0 works
    let found_disabled: bool =
        sqlx::query_scalar::<sqlx::Sqlite, i32>("SELECT 1 FROM mods WHERE id = ? AND status = 0")
            .bind("mod_disabled")
            .fetch_optional(&pool)
            .await
            .unwrap()
            .is_some();
    assert!(
        found_disabled,
        "Manual SQL 'status = 0' should find disabled mod"
    );
}
