use crate::services::config::ConfigService;

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
pub async fn list_mod_folders(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    config: tauri::State<'_, ConfigService>,
    game_id: Option<String>,
    mods_path: String,
    sub_path: Option<String>,
    object_id: Option<String>,
) -> Result<Vec<ModFolder>, String> {
    let folders =
        listing::list_mod_folders_inner(Some(&*pool), game_id, mods_path, sub_path, object_id)
            .await?;
    Ok(helpers::apply_safe_mode_filter(folders, &config))
}

/// Lazily resolve thumbnail for a single mod folder.
/// Called per-card from the frontend after the folder list is rendered.
/// Delegates to ThumbnailCache::resolve() which caps concurrency (4 max),
/// checks folder-keyed L1, and falls back to FS traversal + image processing.
#[tauri::command]
pub async fn get_mod_thumbnail(folder_path: String) -> Result<Option<String>, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    ThumbnailCache::resolve(&folder_path).await
}

/// Delete the thumbnail file for a mod folder (if found) and invalidate cache.
#[tauri::command]
pub async fn delete_mod_thumbnail(folder_path: String) -> Result<(), String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    use crate::services::scanner::core::thumbnail::find_thumbnail;

    let path = std::path::Path::new(&folder_path);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }

    if let Some(thumb_path) = find_thumbnail(path) {
        std::fs::remove_file(&thumb_path)
            .map_err(|e| format!("Failed to delete thumbnail: {}", e))?;
        ThumbnailCache::invalidate(&thumb_path);
    }

    // Always invalidate the folder-keyed cache entry regardless of whether a file was found.
    ThumbnailCache::invalidate_folder(&folder_path);
    Ok(())
}
