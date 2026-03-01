use super::*;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_game_crud() {
    let pool = setup_pool().await;

    let game = GameRow {
        id: "test-id".into(),
        name: "Test Game".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game".into(),
        mod_path: Some("C:\\Mods".into()),
        game_exe: Some("C:\\Game\\game.exe".into()),
        launcher_path: Some("C:\\Loader\\loader.exe".into()),
        loader_exe: Some("C:\\Loader\\loader.exe".into()),
        launch_args: None,
    };

    // Insert
    upsert_game(&pool, &game).await.unwrap();
    let games = get_all_games(&pool).await.unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].name, "Test Game");

    // Update
    let mut updated = game.clone();
    updated.name = "Updated Game".into();
    upsert_game(&pool, &updated).await.unwrap();
    let games = get_all_games(&pool).await.unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].name, "Updated Game");

    // Delete
    delete_game(&pool, "test-id").await.unwrap();
    let games = get_all_games(&pool).await.unwrap();
    assert!(games.is_empty());
}

#[tokio::test]
async fn test_count_games() {
    let pool = setup_pool().await;
    assert_eq!(count_games(&pool).await.unwrap(), 0);

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
    assert_eq!(count_games(&pool).await.unwrap(), 1);
}

#[tokio::test]
async fn test_get_mod_path() {
    let pool = setup_pool().await;
    let game = GameRow {
        id: "g2".into(),
        name: "Game 2".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game2".into(),
        mod_path: Some("C:\\CustomModPath".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    let path = get_mod_path(&pool, "g2").await.unwrap();
    assert_eq!(path.as_deref(), Some("C:\\CustomModPath"));

    let missed = get_mod_path(&pool, "non_existent").await.unwrap();
    assert_eq!(missed, None);
}
