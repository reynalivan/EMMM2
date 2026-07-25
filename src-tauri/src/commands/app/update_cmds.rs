use crate::domain::errors::AppError;
use crate::services::update::{asset_fetch, metadata_sync};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

/// Check for metadata updates from the remote manifest.
///
/// Returns whether an update was applied and the current version.
#[specta::specta]
#[tauri::command]
pub async fn check_metadata_update(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<metadata_sync::MetadataSyncResult, AppError> {
    let result = metadata_sync::check_and_sync_metadata(&pool).await;
    Ok(result)
}

/// Fetch a missing asset file from the remote CDN.
///
/// Returns the local path to the cached asset, or null if the fetch failed.
#[specta::specta]
#[tauri::command]
pub async fn fetch_missing_asset(
    app: AppHandle,
    asset_name: String,
) -> Result<Option<String>, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Failed to get app data dir: {e}")))?;

    let cache_dir = app_data_dir.join("cache");
    let result = asset_fetch::fetch_asset_if_missing(&asset_name, &cache_dir).await;

    Ok(result.map(|p| p.to_string_lossy().to_string()))
}

#[cfg(test)]
#[path = "tests/update_cmds_tests.rs"]
mod tests;
