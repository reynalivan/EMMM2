use crate::database::models::ConfigStatus;
use crate::services::app::app_service::{check_config_status, reset_database_service};
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_check_config_status_fresh() {
    let pool = setup_test_db().await;
    let status = check_config_status(&pool)
        .await
        .expect("Failed to check status");
    assert_eq!(status, ConfigStatus::FreshInstall);
}

#[tokio::test]
async fn test_check_config_status_has_config() {
    let pool = setup_test_db().await;

    // Insert a dummy game
    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'GenshinImpact', '/game/path')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let status = check_config_status(&pool)
        .await
        .expect("Failed to check status");
    assert_eq!(status, ConfigStatus::HasConfig);
}

#[tokio::test]
async fn test_reset_database_service() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let app_data_dir = temp_dir.path();

    // Create a dummy app.db file
    let db_path = app_data_dir.join("app.db");
    fs::write(&db_path, "dummy db content").unwrap();

    // Insert dummy data
    sqlx::query("INSERT INTO app_settings (key, value) VALUES ('theme', 'dark')")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'GenshinImpact', '/game/path')"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Call reset
    reset_database_service(&pool, app_data_dir)
        .await
        .expect("Failed to reset database");

    // Verify backup is created
    let trash_dir = app_data_dir.join("trash");
    assert!(trash_dir.exists());
    let mut backup_found = false;
    for entry in fs::read_dir(&trash_dir).unwrap() {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("app_backup_")
        {
            backup_found = true;
            break;
        }
    }
    assert!(backup_found, "Backup DB was not created in trash folder");

    // Verify DB tables are empty
    let settings_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_settings")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(settings_count, 0, "Settings table should be empty");

    let games_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(games_count, 0, "Games table should be empty");
}
