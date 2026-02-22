use emmm2_lib::services::config::ConfigService;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");

    // Create tables (same as ensure_tables in ConfigService)
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

// Covers: TC-11.1-04 (save/load settings path)
#[tokio::test(flavor = "multi_thread")]
async fn test_config_save_and_load() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool.clone());

    let settings = service.get_settings();
    assert_eq!(settings.theme, "dark");

    let mut next_settings = settings.clone();
    next_settings.theme = "light".to_string();
    service
        .save_settings(next_settings)
        .expect("save should succeed");

    assert_eq!(service.get_settings().theme, "light");

    // Reload from DB via a new service instance
    let service_reloaded = ConfigService::new_for_test(pool);
    assert_eq!(service_reloaded.get_settings().theme, "light");
}

// Covers: TC-11.4-01, DI-11.03 (PIN hash storage + verify)
#[tokio::test(flavor = "multi_thread")]
async fn test_config_pin_operations() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool);

    assert!(service.verify_pin("anything"));

    service
        .set_pin("123456")
        .expect("setting a valid pin should succeed");

    assert!(service.verify_pin("123456"));
    assert!(!service.verify_pin("wrong"));

    // Verify pin_hash is stored but not the raw PIN
    let settings = service.get_settings();
    assert!(settings.safe_mode.pin_hash.is_some());
    let hash = settings.safe_mode.pin_hash.unwrap();
    assert!(!hash.contains("123456"));
}

// Covers: NC-11.4-01, DI-11.04 (5 failed attempts trigger lockout)
#[tokio::test(flavor = "multi_thread")]
async fn test_pin_lockout_after_five_failures() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool);

    service
        .set_pin("123456")
        .expect("setting a valid pin should succeed");

    for _ in 0..4 {
        let status = service.verify_pin_status("000000");
        assert!(!status.valid);
        assert_eq!(status.locked_seconds_remaining, 0);
    }

    let status = service.verify_pin_status("000000");
    assert!(!status.valid);
    assert_eq!(status.attempts_remaining, 0);
    assert!(status.locked_seconds_remaining > 0);
}

// Covers: TC-11.1-04, DI-11.02 (config updates via SQLite remain consistent)
#[tokio::test(flavor = "multi_thread")]
async fn save_settings_can_overwrite_existing() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool);

    let mut first = service.get_settings();
    first.theme = "light".to_string();
    service
        .save_settings(first)
        .expect("first save should succeed");

    let mut second = service.get_settings();
    second.language = "id".to_string();

    let second_save = service.save_settings(second.clone());
    assert!(
        second_save.is_ok(),
        "second save should succeed: {second_save:?}"
    );

    let loaded = service.get_settings();
    assert_eq!(loaded.language, "id");
    assert_eq!(loaded.theme, "light");
}

// Test that games persist through save_settings
#[tokio::test(flavor = "multi_thread")]
async fn test_games_persist_in_db() {
    use emmm2_lib::services::config::GameConfig;
    use std::path::PathBuf;

    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool.clone());

    let mut settings = service.get_settings();
    settings.games.push(GameConfig {
        id: "test-game-1".into(),
        name: "Test Game".into(),
        game_type: "GIMI".into(),
        mod_path: PathBuf::from("C:\\Mods"),
        game_exe: PathBuf::from("C:\\Game\\game.exe"),
        loader_exe: Some(PathBuf::from("C:\\Loader\\loader.exe")),
        launch_args: None,
    });
    service.save_settings(settings).expect("save should succeed");

    // Reload from DB
    let service_reloaded = ConfigService::new_for_test(pool);
    let reloaded = service_reloaded.get_settings();
    assert_eq!(reloaded.games.len(), 1);
    assert_eq!(reloaded.games[0].name, "Test Game");
    assert_eq!(reloaded.games[0].mod_path, PathBuf::from("C:\\Mods"));
}
