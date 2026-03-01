use crate::database::game_repo::{upsert_game, GameRow};
use crate::services::app::dashboard;
use crate::test_utils;

#[tokio::test]
async fn test_dashboard_cmds_delegation() {
    let test_db = test_utils::init_test_db().await;
    let pool = &test_db.pool;

    // Simulate what the command does
    let payload = dashboard::get_dashboard_payload(pool, false).await.unwrap();

    // Asserts
    assert_eq!(payload.stats.total_games, 0);
    assert_eq!(payload.stats.total_mods, 0);

    // Add a game
    upsert_game(
        pool,
        &GameRow {
            id: "fake_game".into(),
            name: "Fake".into(),
            game_type: "GIMI".into(),
            path: "C:\\Fake".into(),
            mod_path: None,
            game_exe: None,
            launcher_path: None,
            loader_exe: None,
            launch_args: None,
        },
    )
    .await
    .unwrap();

    let payload2 = dashboard::get_dashboard_payload(pool, false).await.unwrap();
    assert_eq!(payload2.stats.total_games, 1);
}

#[tokio::test]
async fn test_active_keybindings_delegation() {
    let test_db = test_utils::init_test_db().await;
    let pool = &test_db.pool;

    upsert_game(
        pool,
        &GameRow {
            id: "fake_gen".into(),
            name: "Genshin".into(),
            game_type: "GIMI".into(),
            path: "C:\\Genshin".into(),
            mod_path: None,
            game_exe: None,
            launcher_path: None,
            loader_exe: None,
            launch_args: None,
        },
    )
    .await
    .unwrap();

    let bindings = dashboard::get_active_keybindings_service(pool, "fake_gen")
        .await
        .unwrap();
    assert_eq!(bindings.len(), 0);
}
