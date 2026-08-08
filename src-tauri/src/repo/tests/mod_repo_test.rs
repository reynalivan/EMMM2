use super::*;
use crate::common::path_key::folder_path_key;
use crate::repo::game_repo::{upsert_game, GameRow};
use crate::repo::object_repo::create_object;
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
        game_type: crate::domain::models::GameType::GIMI,
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
    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "mod1",
            game_id: "g1",
            object_id: Some("obj1"),
            actual_name: "My Mod",
            folder_path: "Mods/Obj1/Mod1",
            status: crate::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("C:"),
        },
    )
    .await
    .unwrap();

    // Test get_mod_by_object_id
    let mod_info = get_mod_by_object_id(&pool, "obj1").await.unwrap();
    assert!(mod_info.is_some());
    assert_eq!(mod_info.unwrap().1, "Mods/Obj1/Mod1");

    // Update path and status
    update_mod_path_status_and_reason(
        &pool,
        "g1",
        "Mods/Obj1/Mod1",
        "Mods/Obj1/Mod2",
        crate::domain::models::ItemStatus::Disabled,
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
        game_type: crate::domain::models::GameType::GIMI,
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

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "child_mod",
            game_id: "g_unicode",
            object_id: Some("obj_unicode"),
            actual_name: "日本語Variant",
            folder_path: "한국Character/日本語Root/VariantA",
            status: crate::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("C:"),
        },
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
async fn test_repo_mod_status_consistency() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "g_consist".into(),
        name: "Game Consist".into(),
        game_type: crate::domain::models::GameType::GIMI,
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
    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "mod_enabled",
            game_id: "g_consist",
            object_id: Some("obj_consist"),
            actual_name: "Enabled Mod",
            folder_path: "Mods/EnabledMod",
            status: crate::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("C:"),
        },
    )
    .await
    .unwrap();

    // 2. Insert Disabled mod
    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "mod_disabled",
            game_id: "g_consist",
            object_id: Some("obj_consist"),
            actual_name: "Disabled Mod",
            folder_path: "Mods/DisabledMod",
            status: crate::domain::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("C:"),
        },
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

/// Two games can hold the same mod at the same path relative to their own
/// mods root, which means the same `folder_path_key`. A batch write scoped
/// only by that key hits both games' rows.
async fn seed_same_relative_mod_in_two_games(pool: &SqlitePool) {
    for (game_id, mods_root) in [("g_a", "C:/GameA/Mods"), ("g_b", "C:/GameB/Mods")] {
        upsert_game(
            pool,
            &GameRow {
                id: game_id.into(),
                name: game_id.into(),
                game_type: crate::domain::models::GameType::GIMI,
                path: mods_root.into(),
                mods_path: Some(mods_root.into()),
                game_exe: None,
                launcher_path: None,
                loader_exe: None,
                launch_args: None,
            },
        )
        .await
        .unwrap();

        crate::test_utils::insert_test_mod(
            pool,
            &crate::test_utils::TestModFixture {
                id: &format!("mod_{game_id}"),
                game_id,
                object_id: None,
                actual_name: "Blue Dress",
                // Relative, exactly as disk reconcile stores it.
                folder_path: "Alice/Blue Dress",
                status: crate::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: None,
                mods_path: None,
            },
        )
        .await
        .unwrap();
    }
}

async fn surviving_mod_ids(pool: &SqlitePool) -> Vec<String> {
    sqlx::query_scalar::<_, String>("SELECT id FROM mods ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn batch_delete_only_touches_the_named_game() {
    let pool = setup_pool().await;
    seed_same_relative_mod_in_two_games(&pool).await;

    batch_delete_by_path(&pool, "g_a", &["Alice/Blue Dress".to_string()])
        .await
        .unwrap();

    assert_eq!(
        surviving_mod_ids(&pool).await,
        vec!["mod_g_b".to_string()],
        "deleting in one game must not remove the other game's identically-pathed mod"
    );
}

#[tokio::test]
async fn batch_update_only_touches_the_named_game() {
    let pool = setup_pool().await;
    seed_same_relative_mod_in_two_games(&pool).await;

    batch_update_path_and_status(
        &pool,
        "g_a",
        &[(
            "Alice/Blue Dress".to_string(),
            "Alice/DISABLED Blue Dress".to_string(),
            crate::domain::models::ItemStatus::Disabled,
        )],
    )
    .await
    .unwrap();

    let untouched: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE id = 'mod_g_b'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        untouched, "Alice/Blue Dress",
        "toggling in one game must not rewrite the other game's row"
    );
}
