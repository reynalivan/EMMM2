use super::*;
use crate::repo::game_repo::{upsert_game, GameRow};

#[tokio::test]
async fn get_filtered_objects_matches_scalar_and_array_metadata_case_insensitively() {
    let pool = crate::test_utils::init_test_db().await.pool;

    let game = GameRow {
        id: "g_filters".into(),
        name: "Game Filters".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\GameFilters".into(),
        mods_path: Some("C:\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
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
        search_query: None,
        object_type: Some("Character".to_string()),
        safe_mode: false,
        meta_filters: Some(std::collections::HashMap::from([
            ("element".to_string(), vec!["pyro".to_string()]),
            ("weapon".to_string(), vec!["sword".to_string()]),
        ])),
        sort_by: None,
        status_filter: None,
    };

    let objects = get_filtered_objects(&pool, &filter).await.unwrap();

    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].id, "obj_pyro");
}
