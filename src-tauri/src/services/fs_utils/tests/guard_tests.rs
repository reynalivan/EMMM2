use super::validate_dir_in_configured_roots;
use crate::services::config::{ConfigService, GameConfig};
use std::fs;
use tempfile::TempDir;

async fn config_with_game(mod_path: &std::path::Path) -> ConfigService {
    let pool = crate::test_utils::init_test_db().await.pool;
    let config = ConfigService::new_for_test_async(pool).await;

    let mut settings = config.get_settings();
    settings.games.push(GameConfig {
        id: "game-1".to_string(),
        name: "Test Game".to_string(),
        game_type: crate::domain::models::GameType::GIMI,
        mod_path: mod_path.to_path_buf(),
        game_exe: mod_path.join("game.exe"),
        loader_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    config.save_settings(settings).expect("save settings");
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn accepts_dir_inside_a_configured_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let inside = mods_root.join("Character");
    fs::create_dir_all(&inside).unwrap();

    let config = config_with_game(&mods_root).await;

    let result = validate_dir_in_configured_roots(&config, &inside.to_string_lossy());
    assert!(result.is_ok(), "dir inside the mods root must pass");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn rejects_dir_outside_every_configured_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    fs::create_dir_all(&mods_root).unwrap();
    let outside = tmp.path().join("Elsewhere");
    fs::create_dir_all(&outside).unwrap();

    let config = config_with_game(&mods_root).await;

    let result = validate_dir_in_configured_roots(&config, &outside.to_string_lossy());
    assert!(
        result.is_err(),
        "dir outside the mods root must be rejected"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn rejects_traversal_escaping_the_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    fs::create_dir_all(&mods_root).unwrap();

    let config = config_with_game(&mods_root).await;

    let sneaky = mods_root.join("..");
    let result = validate_dir_in_configured_roots(&config, &sneaky.to_string_lossy());
    assert!(result.is_err(), "`..` escape must be rejected");
}
