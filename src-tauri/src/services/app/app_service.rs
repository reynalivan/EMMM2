//! Application-level status and lifecycle service.
//!
//! Provides `check_config_status` (DB check for fresh install vs configured)
//! and `reset_database_service` (backup + full table clear).

use crate::database::models::ConfigStatus;
use std::path::Path;

/// Determine whether the app has games configured.
/// Returns `HasConfig` when at least one game row exists; `FreshInstall` otherwise.
pub async fn check_config_status(pool: &sqlx::SqlitePool) -> Result<ConfigStatus, String> {
    let count = crate::database::game_repo::count_games(pool)
        .await
        .map_err(|e| format!("Failed to check config status: {e}"))?;

    if count > 0 {
        Ok(ConfigStatus::HasConfig)
    } else {
        Ok(ConfigStatus::FreshInstall)
    }
}

/// Back up `app.db` to the trash folder, then wipe all data from the DB.
/// Does NOT delete any mod files from disk — only clears database records.
pub async fn reset_database_service(
    pool: &sqlx::SqlitePool,
    app_data_dir: &Path,
) -> Result<(), String> {
    let db_path = app_data_dir.join("app.db");
    let trash_dir = app_data_dir.join("trash");

    // Ensure trash directory exists
    if !trash_dir.exists() {
        std::fs::create_dir_all(&trash_dir)
            .map_err(|e| format!("Failed to create trash dir: {e}"))?;
    }

    // Backup the database file with a timestamp
    if db_path.exists() {
        let epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_name = format!("app_backup_{}.db", epoch_secs);
        let backup_path = trash_dir.join(&backup_name);
        std::fs::copy(&db_path, &backup_path)
            .map_err(|e| format!("Failed to backup database: {e}"))?;
        log::info!("Database backed up to: {}", backup_path.display());
    }

    // Clear all data from the database (tables only, no file deletion)
    crate::database::settings_repo::reset_all_data(pool)
        .await
        .map_err(|e| format!("Failed to reset database: {e}"))
}
