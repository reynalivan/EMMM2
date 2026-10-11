//! Application-level status and lifecycle service.
//!
//! Provides `check_config_status` (DB check for fresh install vs configured)
//! and `reset_database_service` (backup + full table clear).

use crate::modules::games::domain::models::ConfigStatus;
use crate::shared::errors::AppError;
use std::path::Path;

fn sqlite_artifact_path(database_path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

pub(super) fn quarantine_database_files(
    database_path: &Path,
    backup_path: &Path,
) -> std::io::Result<()> {
    for suffix in ["", "-wal", "-shm"] {
        let source = sqlite_artifact_path(database_path, suffix);
        if source.exists() {
            std::fs::rename(source, sqlite_artifact_path(backup_path, suffix))?;
        }
    }
    Ok(())
}

pub(super) async fn database_is_corrupt(database_path: &Path) -> bool {
    use sqlx::{Connection, SqliteConnection};

    if !database_path.is_file() {
        return false;
    }
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(database_path)
        .read_only(true);
    let Ok(mut connection) = SqliteConnection::connect_with(&options).await else {
        return false;
    };
    !matches!(
        sqlx::query_scalar::<_, String>("PRAGMA quick_check")
            .fetch_one(&mut connection)
            .await,
        Ok(result) if result == "ok"
    )
}

/// Determine whether the app has games configured.
/// Returns `HasConfig` when at least one game row exists; `FreshInstall` otherwise.
pub async fn check_config_status(pool: &sqlx::SqlitePool) -> Result<ConfigStatus, AppError> {
    let count = crate::modules::games::adapters::sqlite::game::count_games(pool).await?;

    if count > 0 {
        Ok(ConfigStatus::HasConfig)
    } else {
        Ok(ConfigStatus::FreshInstall)
    }
}

/// Back up the current database to the trash folder, then wipe all data from the DB.
/// Does NOT delete any mod files from disk — only clears database records.
pub async fn reset_database_service(
    pool: &sqlx::SqlitePool,
    app_data_dir: &Path,
    settings_revision: Option<u64>,
) -> Result<(), AppError> {
    let db_path = app_data_dir.join("app.db");
    let trash_dir = app_data_dir.join("trash");

    // Ensure trash directory exists
    if !trash_dir.exists() {
        std::fs::create_dir_all(&trash_dir)?;
    }

    // Create a consistent SQLite snapshot, including committed WAL state.
    if db_path.exists() {
        let epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_name = format!("app_backup_{}.db", epoch_secs);
        let backup_path = trash_dir.join(&backup_name);
        sqlx::query("VACUUM INTO ?")
            .bind(backup_path.to_string_lossy().into_owned())
            .execute(pool)
            .await?;
        log::info!("Database backed up to: {}", backup_path.display());
    }

    // Clean up cached image thumbnails
    let thumbnails_dir = app_data_dir.join("thumbnails");
    if thumbnails_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&thumbnails_dir) {
            log::warn!("Failed to delete thumbnails directory: {e}");
        }
    }

    // Clear all data from the database (tables only, no file deletion)
    Ok(
        crate::modules::system::adapters::sqlite::settings::reset_all_data_with_revision(
            pool,
            settings_revision,
        )
        .await?,
    )
}
