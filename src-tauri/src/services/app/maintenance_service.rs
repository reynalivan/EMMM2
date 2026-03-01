//! Application-level maintenance service.
//!
//! Extracts the vacuum-db + prune-thumbnails + orphan-collection-cleanup +
//! trash-purge orchestration that was previously inlined in `settings_cmds.rs`.

use std::path::Path;
use std::time::{Duration, SystemTime};

use sqlx::SqlitePool;

/// Run all maintenance tasks and return a human-readable summary string.
pub async fn run_maintenance(pool: &SqlitePool, app_data_dir: &Path) -> Result<String, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;

    // 1. Vacuum DB
    crate::database::settings_repo::vacuum_database(pool)
        .await
        .map_err(|e| format!("VACUUM failed: {}", e))?;

    // 2. Prune orphaned thumbnails
    let valid_paths = crate::database::settings_repo::get_all_thumbnail_paths(pool)
        .await
        .map_err(|e| format!("Failed to fetch thumbnails: {}", e))?;

    let pruned_count =
        ThumbnailCache::prune_orphans(&valid_paths).map_err(|e| format!("Prune failed: {}", e))?;

    // 3. Remove orphaned collection_items rows
    let orphan_rows_affected =
        crate::database::settings_repo::remove_orphaned_collection_items(pool)
            .await
            .map_err(|e| format!("Orphan cleanup failed: {e}"))?;

    // 4. Purge empty trash entries older than 30 days
    let trash_dir = app_data_dir.join("trash");
    let purged_trash_count = cleanup_old_empty_trash_entries(&trash_dir)?;

    Ok(format!(
        "Maintenance complete. Database optimized. Pruned {} thumbnails. Removed {} orphaned collection rows. Purged {} old empty trash entries.",
        pruned_count, orphan_rows_affected, purged_trash_count
    ))
}

pub fn cleanup_old_empty_trash_entries(trash_dir: &Path) -> Result<u64, String> {
    if !trash_dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(30 * 24 * 60 * 60))
        .ok_or_else(|| "Failed to compute cleanup cutoff".to_string())?;

    let mut removed = 0_u64;
    for entry in std::fs::read_dir(trash_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let metadata_path = path.join("metadata.json");
        if metadata_path.exists() {
            continue;
        }

        let modified = match entry.metadata().and_then(|meta| meta.modified()) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if modified >= cutoff {
            continue;
        }

        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        removed = removed.saturating_add(1);
    }

    Ok(removed)
}
