use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;

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
) -> Result<Option<String>, AppError> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    // Fortify Safe Mode: Do not serve thumbnails for unsafe mods if Safe Mode is locked (enabled)
    if config.current_corridor().is_safe() {
        let analysis = crate::services::explorer::helpers::analyze_mod_metadata(
            std::path::Path::new(&folder_path),
            None,
        );
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
    watcher: tauri::State<'_, WatcherState>,
    folder_path: String,
) -> Result<(), AppError> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    use crate::services::scanner::core::thumbnail::find_thumbnail;

    let path = std::path::Path::new(&folder_path);
    if !path.exists() {
        return Err(AppError::NotFound("Folder does not exist".to_string()));
    }

    let settings = config.get_settings();
    let game_id = settings
        .games
        .iter()
        .find(|game| path.starts_with(&game.mod_path))
        .map(|game| game.id.clone())
        .ok_or_else(|| {
            AppError::Security("Folder is outside every configured mods_path".to_string())
        })?;

    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let mut changed_paths: Vec<String> = Vec::new();
    if let Some(thumb_path) = find_thumbnail(path) {
        crate::services::fs_utils::recycle_bin::move_path_to_recycle_bin(&thumb_path)?;
        ThumbnailCache::invalidate(&thumb_path);
        changed_paths.push(thumb_path.to_string_lossy().to_string());
    }

    // Always invalidate the folder-keyed cache entry regardless of whether a file was found.
    ThumbnailCache::invalidate_folder(&folder_path);
    if changed_paths.is_empty() {
        changed_paths.push(folder_path.clone());
    }

    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, changed_paths).await
}
