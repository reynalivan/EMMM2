// use super::*;
use crate::services::config::ConfigService;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

async fn setup_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    pool
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
    assert_eq!(results[0].game_type, "GIMI");

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
async fn test_remove_game_cascading() {
    let pool = setup_pool().await;
    let service = ConfigService::new_for_test(pool.clone());

    let tmp = TempDir::new().unwrap();
    let game_dir = tmp.path().join("MyGimi");
    create_valid_instance(&game_dir);

    let game = crate::commands::app::game_cmds::add_game_manual_inner(
        &service,
        "GIMI",
        &game_dir.to_string_lossy(),
    )
    .await
    .unwrap();

    // Check it's in DB
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE id = ?")
        .bind(&game.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Remove it
    crate::services::game::game_service::remove_game_service(&service, &game.id)
        .await
        .unwrap();

    // Check it's removed from settings memory struct
    let settings = service.get_settings();
    assert!(settings.games.is_empty());

    // Check it's removed from DB
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE id = ?")
        .bind(&game.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
