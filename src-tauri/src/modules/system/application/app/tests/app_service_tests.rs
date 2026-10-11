use crate::modules::games::domain::models::ConfigStatus;
use crate::modules::games::domain::models::GameType;
use crate::modules::system::application::app::app_service::{
    check_config_status, database_is_corrupt, quarantine_database_files, reset_database_service,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
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
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: GameType::Gimi,
            path: "/game/path",
            mods_path: Some("/game/path/Mods"),
        },
    )
    .await
    .unwrap();

    let status = check_config_status(&pool)
        .await
        .expect("Failed to check status");
    assert_eq!(status, ConfigStatus::HasConfig);
}

#[tokio::test]
async fn test_reset_database_service() {
    let temp_dir = TempDir::new().unwrap();
    let app_data_dir = temp_dir.path();
    let db_path = app_data_dir.join("app.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint = 0")
        .execute(&pool)
        .await
        .unwrap();

    // Insert dummy data
    sqlx::query("INSERT INTO app_settings (key, value) VALUES ('theme', 'dark')")
        .execute(&pool)
        .await
        .unwrap();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: GameType::Gimi,
            path: "/game/path",
            mods_path: Some("/game/path/Mods"),
        },
    )
    .await
    .unwrap();

    // Call reset
    reset_database_service(&pool, app_data_dir, None)
        .await
        .expect("Failed to reset database");

    // Verify backup is created
    let trash_dir = app_data_dir.join("trash");
    assert!(trash_dir.exists());
    let backup_path = fs::read_dir(&trash_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("app_backup_"))
        })
        .expect("Backup DB was not created in trash folder");
    let backup_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(backup_path)
                .read_only(true),
        )
        .await
        .expect("backup should open as a standalone SQLite database");
    let backed_up_theme: String =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'theme'")
            .fetch_one(&backup_pool)
            .await
            .expect("backup should retain data committed before reset");
    assert_eq!(backed_up_theme, "dark");

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

#[test]
fn quarantine_preserves_database_and_wal_sidecars() {
    let temp = TempDir::new().expect("temporary database root should create");
    let database = temp.path().join("app.db");
    let backup = temp.path().join("app_corrupt.db");
    for suffix in ["", "-wal", "-shm"] {
        fs::write(format!("{}{}", database.display(), suffix), suffix)
            .expect("database artifact should write");
    }

    quarantine_database_files(&database, &backup).expect("database artifacts should move");

    for suffix in ["", "-wal", "-shm"] {
        assert!(!std::path::PathBuf::from(format!("{}{}", database.display(), suffix)).exists());
        assert!(std::path::PathBuf::from(format!("{}{}", backup.display(), suffix)).exists());
    }
}

#[tokio::test]
async fn healthy_database_is_not_classified_as_corrupt() {
    let temp = TempDir::new().expect("temporary database root should create");
    let database = temp.path().join("app.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true),
        )
        .await
        .expect("healthy database should open");
    sqlx::query("CREATE TABLE test_value (value TEXT)")
        .execute(&pool)
        .await
        .expect("healthy database should accept a write");
    pool.close().await;

    assert!(!database_is_corrupt(&database).await);
}
