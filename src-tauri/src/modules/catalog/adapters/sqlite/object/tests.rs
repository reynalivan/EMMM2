use super::*;
use crate::modules::catalog::domain::objects::ObjectFilter;
use crate::modules::games::adapters::sqlite::game::{upsert_game, GameRow};
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::test_utils::{
    insert_test_game, insert_test_mod, insert_test_object, TestGameFixture, TestModFixture,
    TestObjectFixture,
};

#[tokio::test]
async fn get_filtered_objects_matches_scalar_and_array_metadata_case_insensitively() {
    let pool = crate::test_utils::init_test_db().await.pool;

    let game = GameRow {
        id: "g_filters".into(),
        name: "Game Filters".into(),
        game_type: crate::modules::games::domain::models::GameType::Gimi,
        path: "C:\\GameFilters".into(),
        mods_path: Some("C:\\Mods".into()),
        ready_to_move_path: None,
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_mode: "standalone".into(),
        xxmi_launcher_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    create_object(
        &pool,
        "obj_pyro",
        "g_filters",
        "Pyro Sword",
        "Pyro Sword",
        "Character",
        None,
        None,
        r#"{"element":"Pyro","weapon":["Sword","Claymore"],"rarity":5}"#,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    create_object(
        &pool,
        "obj_hydro",
        "g_filters",
        "Hydro Bow",
        "Hydro Bow",
        "Character",
        None,
        None,
        r#"{"element":"Hydro","weapon":["Bow"],"rarity":4}"#,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let filter = ObjectFilter {
        game_id: "g_filters".to_string(),
        object_type: Some("Character".to_string()),
        meta_filters: Some(std::collections::HashMap::from([
            ("element".to_string(), vec!["pyro".to_string()]),
            ("weapon".to_string(), vec!["sword".to_string()]),
        ])),
        ..Default::default()
    };

    let page = get_filtered_objects(&pool, &filter).await.unwrap();

    assert_eq!(page.objects.len(), 1);
    assert_eq!(page.objects[0].id, "obj_pyro");
}

#[tokio::test]
async fn deleting_object_treats_sql_like_wildcards_in_its_folder_as_literal() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let mods_path = "C:/Mods";
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game-like-delete",
            name: "LIKE delete",
            game_type: GameType::Gimi,
            path: "C:/Game",
            mods_path: Some(mods_path),
        },
    )
    .await
    .expect("seed game");
    for (id, folder_path) in [
        ("object-pattern-underscore", "A_B"),
        ("object-sibling-underscore", "AcB"),
        ("object-pattern-percent", "P%B"),
        ("object-sibling-percent", "PqB"),
    ] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id,
                game_id: "game-like-delete",
                name: folder_path,
                folder_path,
                object_type: "Character",
            },
        )
        .await
        .expect("seed object");
    }
    for (id, object_id, folder_path) in [
        (
            "mod-pattern-underscore",
            "object-pattern-underscore",
            "A_B/Blue",
        ),
        (
            "mod-sibling-underscore",
            "object-sibling-underscore",
            "AcB/Red",
        ),
        ("mod-pattern-percent", "object-pattern-percent", "P%B/Blue"),
        ("mod-sibling-percent", "object-sibling-percent", "PqB/Red"),
    ] {
        insert_test_mod(
            &pool,
            &TestModFixture {
                id,
                game_id: "game-like-delete",
                object_id: Some(object_id),
                actual_name: id,
                folder_path,
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(mods_path),
            },
        )
        .await
        .expect("seed mod");
    }

    let deleted = {
        let mut connection = pool.acquire().await.expect("acquire connection");
        delete_object_and_mods_by_folder(&mut connection, "game-like-delete", "A_B")
            .await
            .expect("delete object")
    };

    assert_eq!(deleted, 1, "underscore is a literal path character");
    let remaining_mod_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM mods ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("read remaining mods");
    assert_eq!(
        remaining_mod_ids,
        vec![
            "mod-pattern-percent",
            "mod-sibling-percent",
            "mod-sibling-underscore"
        ]
    );

    let deleted = {
        let mut connection = pool.acquire().await.expect("acquire connection");
        delete_object_and_mods_by_folder(&mut connection, "game-like-delete", "P%B")
            .await
            .expect("delete percent object")
    };

    assert_eq!(deleted, 1, "percent is a literal path character");
    let remaining_mod_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM mods ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("read remaining mods");
    assert_eq!(
        remaining_mod_ids,
        vec!["mod-sibling-percent", "mod-sibling-underscore"]
    );
}
