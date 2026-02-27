use crate::services::update::{asset_fetch, metadata_sync};
use crate::types::errors::{CommandError, CommandResult};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

/// Check for metadata updates from the remote manifest.
///
/// Returns whether an update was applied and the current version.
#[tauri::command]
pub async fn check_metadata_update(
    pool: tauri::State<'_, SqlitePool>,
) -> CommandResult<metadata_sync::MetadataSyncResult> {
    let result = metadata_sync::check_and_sync_metadata(&pool).await;
    Ok(result)
}

/// Fetch a missing asset file from the remote CDN.
///
/// Returns the local path to the cached asset, or null if the fetch failed.
#[tauri::command]
pub async fn fetch_missing_asset(
    app: AppHandle,
    asset_name: String,
) -> CommandResult<Option<String>> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| CommandError::Internal(format!("Failed to get app data dir: {e}")))?;

    let cache_dir = app_data_dir.join("cache");
    let result = asset_fetch::fetch_asset_if_missing(&asset_name, &cache_dir).await;

    Ok(result.map(|p| p.to_string_lossy().to_string()))
}
