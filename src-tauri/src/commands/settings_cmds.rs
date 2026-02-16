use crate::services::config::{pin_guard::PinVerifyStatus, AppSettings, ConfigService};
use tauri::State;
use std::time::{Duration, SystemTime};

#[tauri::command]
pub async fn get_settings(state: State<'_, ConfigService>) -> Result<AppSettings, String> {
    Ok(state.get_settings())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, ConfigService>,
) -> Result<(), String> {
    state.save_settings(settings)
}

#[tauri::command]
pub async fn set_safe_mode_pin(pin: String, state: State<'_, ConfigService>) -> Result<(), String> {
    state.set_pin(&pin)
}

#[tauri::command]
pub async fn verify_pin(pin: String, state: State<'_, ConfigService>) -> Result<PinVerifyStatus, String> {
    Ok(state.verify_pin_status(&pin))
}

#[tauri::command]
pub async fn set_active_game(game_id: Option<String>, state: State<'_, ConfigService>) -> Result<(), String> {
    state.set_active_game(game_id)
}

#[tauri::command]
pub async fn set_safe_mode_enabled(enabled: bool, state: State<'_, ConfigService>) -> Result<(), String> {
    state.set_safe_mode_enabled(enabled)
}

#[tauri::command]
pub async fn run_maintenance(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<String, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    use tauri::Manager;
    use sqlx::Row;

    // 1. Vacuum DB (Optimize storage)
    sqlx::query("VACUUM")
        .execute(pool.inner())
        .await
        .map_err(|e| format!("VACUUM failed: {}", e))?;

    // 2. Prune Thumbnails
    // Get all valid image paths from DB
    let rows = sqlx::query("SELECT DISTINCT thumbnail_path FROM mods WHERE thumbnail_path IS NOT NULL")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| format!("Failed to fetch thumbnails: {}", e))?;

    let valid_paths: Vec<String> = rows
        .iter()
        .map(|r| r.get("thumbnail_path"))
        .collect();

    let pruned_count = ThumbnailCache::prune_orphans(&valid_paths)
        .map_err(|e| format!("Prune failed: {}", e))?;

    // 3. Remove orphaned collection_items rows
    let orphan_rows = sqlx::query(
        "DELETE FROM collection_items
         WHERE collection_id NOT IN (SELECT id FROM collections)
            OR mod_id NOT IN (SELECT id FROM mods)",
    )
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Orphan cleanup failed: {e}"))?;

    // 4. Purge empty trash entries older than 30 days
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let trash_dir = app_data_dir.join("trash");
    let purged_trash_count = cleanup_old_empty_trash_entries(&trash_dir)?;

    Ok(format!(
        "Maintenance complete. Database optimized. Pruned {} thumbnails. Removed {} orphaned collection rows. Purged {} old empty trash entries.",
        pruned_count,
        orphan_rows.rows_affected(),
        purged_trash_count
    ))
}

fn cleanup_old_empty_trash_entries(trash_dir: &std::path::Path) -> Result<u64, String> {
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
