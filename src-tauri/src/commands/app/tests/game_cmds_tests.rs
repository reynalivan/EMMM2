// use super::*;
use crate::services::config::ConfigService;

use std::fs;
use std::path::Path;
use tempfile::TempDir;

async fn setup_pool() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

// Minimal valid 3DMigoto folder requirement
fn create_valid_instance(dir: &Path) {
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "[Constants]").unwrap();
    fs::write(dir.join("d3d11.dll"), "fake-dll").unwrap();
    fs::write(dir.join("3DMigotoLoader.exe"), "fake-exe").unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_auto_detect_games() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool);

    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("XXMI");
    fs::create_dir_all(&root).unwrap();

    // Create valid GIMI
    create_valid_instance(&root.join("GIMI"));

    let results =
        crate::commands::app::game_cmds::auto_detect_games_inner(&service, &root.to_string_lossy())
            .await
            .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].game_type,
        crate::database::models::GameType::GIMI
    );

    // Persist to DB/Settings
    crate::commands::app::game_cmds::save_onboarding_games_inner(&service, results)
        .await
        .unwrap();

    // DB verification
    let settings = service.get_settings();
    assert_eq!(settings.games.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_add_game_manual_and_duplicate() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool);

    let tmp = TempDir::new().unwrap();
    let game_dir = tmp.path().join("MyGimi");
    create_valid_instance(&game_dir);

    // Initial add
    let result = crate::commands::app::game_cmds::add_game_manual_inner(
        &service,
        "GIMI",
        &game_dir.to_string_lossy(),
    )
    .await;
    assert!(result.is_ok());
    let game = result.unwrap();

    // Persist
    crate::commands::app::game_cmds::save_onboarding_games_inner(&service, vec![game])
        .await
        .unwrap();

    // Duplicate add should fail
    let dup_result = crate::commands::app::game_cmds::add_game_manual_inner(
        &service,
        "GIMI",
        &game_dir.to_string_lossy(),
    )
    .await;
    assert!(dup_result.is_err());
    assert!(
        dup_result.unwrap_err().contains("already registered"),
        "Duplicate addition was not prevented."
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_add_game_manual_rejects_unicode_duplicate_with_ascii_case_and_slash_variants() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool);

    let tmp = TempDir::new().unwrap();
    let game_dir = tmp.path().join("My日本語GIMI");
    create_valid_instance(&game_dir);

    let game = crate::commands::app::game_cmds::add_game_manual_inner(
        &service,
        "GIMI",
        &game_dir.to_string_lossy(),
    )
    .await
    .unwrap();

    crate::commands::app::game_cmds::save_onboarding_games_inner(&service, vec![game])
        .await
        .unwrap();

    let duplicate_variant = game_dir
        .to_string_lossy()
        .replace('\\', "/")
        .replace("My", "my")
        .replace("GIMI", "gimi");

    let dup_result = crate::commands::app::game_cmds::add_game_manual_inner(
        &service,
        "GIMI",
        &duplicate_variant,
    )
    .await;

    assert!(dup_result.is_err());
    assert!(dup_result.unwrap_err().contains("already registered"));
}
