use std::path::Path;

use crate::services::config::ConfigService;
use crate::DISABLED_PREFIX;

pub mod helpers;
pub mod listing;
#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
pub mod types;

// Re-export specific items for external modules
// try_resolve_alternate used by conflict resolver or archive? Need to re-export it.
pub(crate) use helpers::try_resolve_alternate;

pub use types::ModFolder;

// ── Public commands ───────────────────────────────────────────────────────────

/// Bulk-check which folder names physically exist as directories under `base_dir`.
/// Returns only the names that exist. Used to filter ObjectList by filesystem truth.
#[tauri::command]
pub async fn filter_existing_folders(
    base_dir: String,
    folder_names: Vec<String>,
) -> Result<Vec<String>, String> {
    let base = Path::new(&base_dir);
    if !base.exists() || !base.is_dir() {
        return Ok(Vec::new());
    }

    let existing: Vec<String> = folder_names
        .into_iter()
        .filter(|name| {
            let path = base.join(name);
            let disabled_path = base.join(format!("{}{}", DISABLED_PREFIX, name));
            (path.exists() && path.is_dir()) || (disabled_path.exists() && disabled_path.is_dir())
        })
        .collect();

    Ok(existing)
}

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
