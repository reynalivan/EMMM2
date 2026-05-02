use crate::services::config::ConfigService;
use crate::services::disk_reconcile::orchestrator::DiskReconcileState;
use crate::services::disk_reconcile::types::DiskReconcileReason;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use tauri::Emitter;

pub mod classifier;
pub mod helpers;
pub mod listing;
#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
pub mod types;

pub use types::ModFolder;

/// List mod folders at a given path, optionally navigating into a sub_path.
///
/// - `mods_path`: The root mods directory for the game.
/// - `sub_path`: Optional relative sub-path for deep navigation (e.g., "Raiden/Set1").
///
/// Returns folder entries with enabled/disabled state, thumbnails, metadata.
/// Covers: TC-4.1-01 (Deep Navigation), TC-4.1-02 (Sort by Date)
#[tauri::command]
#[specta::specta]
pub async fn list_mod_folders(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    mods_path: String,
    sub_path: Option<String>,
    _object_id: Option<String>,
) -> Result<types::FolderGridResponse, String> {
    let response =
        listing::list_mod_folders_for_game(pool.inner(), &game_id, mods_path, sub_path).await?;
    Ok(helpers::apply_safe_mode_filter_to_response(
        response, &config,
    ))
}

/// Lazily resolve thumbnail for a single mod folder.
/// Called per-card from the frontend after the folder list is rendered.
/// Delegates to ThumbnailCache::resolve() which caps concurrency (4 max),
/// checks folder-keyed L1, and falls back to FS traversal + image processing.
#[tauri::command]
#[specta::specta]
pub async fn get_mod_thumbnail(
    game_id: String,
    folder_path: String,
    config: tauri::State<'_, ConfigService>,
) -> Result<Option<String>, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    // Fortify Safe Mode: Do not serve thumbnails for unsafe mods if Safe Mode is locked (enabled)
    if config.get_settings().safe_mode.enabled {
        let analysis = helpers::analyze_mod_metadata(std::path::Path::new(&folder_path), None);
        if !analysis.is_safe {
            return Ok(None);
        }
    }

    ThumbnailCache::resolve(&game_id, &folder_path).await
}

/// Delete the thumbnail file for a mod folder (if found) and invalidate cache.
#[tauri::command]
#[specta::specta]
pub async fn delete_mod_thumbnail(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: tauri::State<'_, DiskReconcileState>,
    watcher: tauri::State<'_, WatcherState>,
    folder_path: String,
) -> Result<(), String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    use crate::services::scanner::core::thumbnail::find_thumbnail;

    let path = std::path::Path::new(&folder_path);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }

    let settings = config.get_settings();
    let game_id = settings
        .games
        .iter()
        .find(|game| path.starts_with(&game.mod_path))
        .map(|game| game.id.clone())
        .ok_or_else(|| "Folder is outside every configured mods_path".to_string())?;

    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let mut changed_paths: Vec<String> = Vec::new();
    if let Some(thumb_path) = find_thumbnail(path) {
        std::fs::remove_file(&thumb_path)
            .map_err(|e| format!("Failed to delete thumbnail: {}", e))?;
        ThumbnailCache::invalidate(&thumb_path);
        changed_paths.push(thumb_path.to_string_lossy().to_string());
    }

    // Always invalidate the folder-keyed cache entry regardless of whether a file was found.
    ThumbnailCache::invalidate_folder(&folder_path);
    if changed_paths.is_empty() {
        changed_paths.push(folder_path.clone());
    }

    let result = crate::services::disk_reconcile::orchestrator::reconcile_disk_state(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        watcher.suppressor.clone(),
        game_id,
        DiskReconcileReason::InternalMutation,
        changed_paths,
        false,
    )
    .await?;

    app.emit("disk_reconcile:result", result)
        .map_err(|error| error.to_string())?;
    Ok(())
}
