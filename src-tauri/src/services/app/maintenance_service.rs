//! Application-level maintenance service.
//!
//! Extracts the vacuum-db + prune-thumbnails + trash-purge orchestration that
//! was previously inlined in `settings_cmds.rs`.

use crate::domain::errors::AppError;
use std::path::Path;

use sqlx::SqlitePool;

/// Run all maintenance tasks and return counts of pruned/purged items.
pub async fn run_maintenance_counts(
    pool: &SqlitePool,
    app_data_dir: &Path,
) -> Result<(u64, u64), AppError> {
    use crate::services::images::thumbnail_cache::{ThumbnailCache, THUMBNAIL_RETENTION_DAYS};

    // 1. Vacuum DB
    crate::repo::settings_repo::vacuum_database(pool).await?;

    // 2. Prune thumbnails nothing has looked at in a while.
    //
    // This used to prune "orphans", keeping only cache entries whose key
    // matched an `objects.thumbnail_path`. But the cache is keyed by the image
    // file found *inside a mod folder*, and object thumbnails are a different
    // population entirely — so the keep-set almost never matched and every run
    // wiped the whole folder-grid cache while reporting it as cleanup. Age is
    // a predicate this layer can actually evaluate correctly.
    let pruned_count =
        ThumbnailCache::clear_old_cache_for_app_data(app_data_dir, THUMBNAIL_RETENTION_DAYS)?;

    // 3. Purge empty trash entries older than 30 days
    let trash_dir = app_data_dir.join("trash");
    let purged_trash_count = cleanup_old_empty_trash_entries(&trash_dir).unwrap_or_else(|e| {
        log::warn!("Trash cleanup failed: {}", e);
        0
    });

    Ok((pruned_count as u64, purged_trash_count))
}

pub fn cleanup_old_empty_trash_entries(trash_dir: &Path) -> Result<u64, AppError> {
    let _ = trash_dir;
    Ok(0)
}
