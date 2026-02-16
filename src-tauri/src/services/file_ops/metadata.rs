use crate::services::images::thumbnail_cache::ThumbnailCache;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;

/// Set the category (Object Type) for a mod.
/// Updates the `mods` table.
pub async fn set_mod_category(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    category: &str,
) -> Result<(), String> {
    // Check if mod exists
    let exists = sqlx::query("SELECT id FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    if exists.is_some() {
        sqlx::query("UPDATE mods SET object_type = ? WHERE folder_path = ? AND game_id = ?")
            .bind(category)
            .bind(folder_path)
            .bind(game_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        return Err("Mod not found in database. Please sync first.".to_string());
    }

    Ok(())
}

/// Update the thumbnail for a mod folder.
/// Copies the source image to `preview.png` (or keeps extension) in the mod folder.
/// Invalidates cache.
pub fn update_mod_thumbnail(folder_path: &str, source_path: &str) -> Result<String, String> {
    let target_dir = Path::new(folder_path);
    let source_path_obj = Path::new(source_path);

    if !target_dir.exists() || !target_dir.is_dir() {
        return Err(format!("Target folder does not exist: {folder_path}"));
    }
    if !source_path_obj.exists() || !source_path_obj.is_file() {
        return Err(format!("Source file does not exist: {source_path}"));
    }

    // Determine the new thumbnail path within the mod folder
    let new_thumbnail_name = source_path_obj
        .file_name()
        .ok_or("Invalid source file name")?
        .to_string_lossy()
        .to_string();
    let new_thumbnail_path = target_dir.join(&new_thumbnail_name);

    // Copy the source image to the mod folder
    fs::copy(source_path_obj, &new_thumbnail_path)
        .map_err(|e| format!("Failed to copy thumbnail: {e}"))?;

    // Invalidate cache for this mod's thumbnail
    ThumbnailCache::invalidate(&new_thumbnail_path);

    Ok(new_thumbnail_path.to_string_lossy().to_string())
}
