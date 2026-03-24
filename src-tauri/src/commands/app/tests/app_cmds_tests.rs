use super::*;
use crate::repo::game_repo::{upsert_game, GameRow};
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_check_config_status_fresh_install() {
    let pool = setup_pool().await;

    // With 0 games, it should return FreshInstall
    let status = crate::services::app::app_service::check_config_status(&pool)
        .await
        .unwrap();
    assert_eq!(status, ConfigStatus::FreshInstall);
}

#[tokio::test]
async fn test_check_config_status_has_config() {
    let pool = setup_pool().await;

    // Insert 1 game
    let game = GameRow {
        id: "game1".into(),
        name: "Test Game".into(),
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\Game".into(),
        mods_path: Some("C:\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // With 1 game, it should return HasConfig
    let status = crate::services::app::app_service::check_config_status(&pool)
        .await
        .unwrap();
    assert_eq!(status, ConfigStatus::HasConfig);
}
