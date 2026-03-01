use super::*;
use crate::database::game_repo::{upsert_game, GameRow};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_type TEXT NOT NULL,
            path TEXT NOT NULL,
            launcher_path TEXT,
            launch_args TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            mod_path TEXT,
            game_exe TEXT,
            loader_exe TEXT
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
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
        game_type: "GIMI".into(),
        path: "C:\\Game".into(),
        mod_path: None,
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
