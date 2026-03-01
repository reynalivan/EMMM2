use super::*;
use crate::database::game_repo::{upsert_game, GameRow};
use crate::database::object_repo::create_object;
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

    // Insert Mod
    insert_new_mod(&pool, "mod1", "g1", "My Mod", "Mods/Obj1/Mod1", "ENABLED")
        .await
        .unwrap();

    // Set mod object id manually
    set_mod_object(&pool, "mod1", "obj1").await.unwrap();

    // Test get_object_id_by_path
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
    update_mod_path_and_status(&pool, "g1", "Mods/Obj1/Mod1", "Mods/Obj1/Mod2", "DISABLED")
        .await
        .unwrap();

    // Test delete by id
    delete_mod_by_id(&pool, "mod1").await.unwrap();
    let mod_info2 = get_mod_by_object_id(&pool, "obj1").await.unwrap();
    assert!(mod_info2.is_none());
}
