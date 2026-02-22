use super::*;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");

    // Run the minimal schema needed for tests
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

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
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

/// Creates an in-memory pool with all tables needed by `reset_all_data`.
async fn setup_pool_full() -> SqlitePool {
    let pool = setup_pool().await;

    let extra_tables = [
        "CREATE TABLE IF NOT EXISTS mods (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            actual_name TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            status TEXT DEFAULT 'DISABLED',
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS objects (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            name TEXT NOT NULL,
            folder_path TEXT
        )",
        "CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_id TEXT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS collection_items (
            collection_id TEXT NOT NULL,
            mod_id TEXT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS scan_results (
            id TEXT PRIMARY KEY
        )",
        "CREATE TABLE IF NOT EXISTS dedup_jobs (
            id TEXT PRIMARY KEY
        )",
        "CREATE TABLE IF NOT EXISTS dedup_groups (
            id TEXT PRIMARY KEY
        )",
        "CREATE TABLE IF NOT EXISTS dedup_group_members (
            id TEXT PRIMARY KEY
        )",
        "CREATE TABLE IF NOT EXISTS duplicate_whitelist (
            id TEXT PRIMARY KEY
        )",
    ];

    for ddl in extra_tables {
        sqlx::query(ddl).execute(&pool).await.unwrap();
    }

    pool
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
