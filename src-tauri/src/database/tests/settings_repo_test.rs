use super::*;
use crate::database::game_repo::{count_games, get_all_games, upsert_game, GameRow};
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_kv_setting_round_trip() {
    let pool = setup_pool().await;

    // Initially empty
    let val = get_setting(&pool, "theme").await.unwrap();
    assert!(val.is_none());

    // Set value
    set_setting(&pool, "theme", "dark").await.unwrap();
    let val = get_setting(&pool, "theme").await.unwrap();
    assert_eq!(val.as_deref(), Some("dark"));

    // Overwrite
    set_setting(&pool, "theme", "light").await.unwrap();
    let val = get_setting(&pool, "theme").await.unwrap();
    assert_eq!(val.as_deref(), Some("light"));
}

#[tokio::test]
async fn test_get_all_settings() {
    let pool = setup_pool().await;

    set_setting(&pool, "theme", "dark").await.unwrap();
    set_setting(&pool, "language", "en").await.unwrap();

    let all = get_all_settings(&pool).await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get("theme").map(|s| s.as_str()), Some("dark"));
    assert_eq!(all.get("language").map(|s| s.as_str()), Some("en"));
}

/// Creates an in-memory pool with all tables needed by `reset_all_data`.
async fn setup_pool_full() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_reset_all_data() {
    let pool = setup_pool_full().await;

    // Seed data
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
    set_setting(&pool, "theme", "dark").await.unwrap();
    set_setting(&pool, "language", "en").await.unwrap();

    // Pre-condition
    assert_eq!(count_games(&pool).await.unwrap(), 1);
    assert_eq!(get_all_settings(&pool).await.unwrap().len(), 2);

    // Act
    reset_all_data(&pool).await.unwrap();

    // Assert
    assert_eq!(count_games(&pool).await.unwrap(), 0);
    assert!(get_all_settings(&pool).await.unwrap().is_empty());
    assert!(get_all_games(&pool).await.unwrap().is_empty());
}
