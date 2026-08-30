use emmm_lib::modules::settings::application::config::ConfigService;
use sqlx::SqlitePool;

mod common;

async fn setup_pool() -> SqlitePool {
    let ctx = common::init_test_db().await;
    ctx.pool
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
    use emmm_lib::modules::settings::application::config::GameConfig;
    use std::path::PathBuf;

    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool.clone());

    let mut settings = service.get_settings();
    settings.games.push(GameConfig {
        id: "test-game-1".into(),
        name: "Test Game".into(),
        game_type: emmm_lib::modules::games::domain::models::GameType::GIMI,
        mod_path: PathBuf::from("C:\\Mods"),
        ready_to_move_path: None,
        game_exe: PathBuf::from("C:\\Game\\game.exe"),
        loader_exe: Some(PathBuf::from("C:\\Loader\\loader.exe")),
        launch_args: None,
        warnings: vec![],
    });
    service
        .save_settings(settings)
        .expect("save should succeed");

    // Reload from DB
    let service_reloaded = ConfigService::new_for_test(pool);
    let reloaded = service_reloaded.get_settings();
    assert_eq!(reloaded.games.len(), 1);
    assert_eq!(reloaded.games[0].name, "Test Game");
    assert_eq!(reloaded.games[0].mod_path, PathBuf::from("C:\\Mods"));
}
