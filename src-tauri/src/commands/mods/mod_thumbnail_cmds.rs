use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::fs_utils::guard::validate_path;
use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::mods::metadata;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

#[specta::specta]
#[tauri::command]
pub async fn update_mod_thumbnail(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    source_path: String,
) -> Result<String, AppError> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let abs_path = metadata::update_mod_thumbnail(&config, &game_id, &folder_path, &source_path)?;
    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, vec![abs_path.clone()])
        .await
        .map_err(AppError::Internal)?;

    // Return the absolute path directly
    Ok(abs_path)
}

#[specta::specta]
#[tauri::command]
pub async fn paste_thumbnail(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let saved_path =
        paste_thumbnail_inner(&config, game_id.clone(), folder_path, image_data).await?;
    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, vec![saved_path.clone()])
        .await
        .map_err(AppError::Internal)?;
    Ok(saved_path)
}

pub async fn paste_thumbnail_inner(
    config: &ConfigService,
    game_id: String,
    folder_path: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    use image::ImageFormat;
    const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

    if image_data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Metadata(
            crate::domain::errors::MetadataError::Validation(
                "Image too large. Max 10MB.".to_string(),
            ),
        ));
    }

    let path = validate_path(config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(crate::domain::errors::MetadataError::Security(e)))?;

    let img = image::load_from_memory(&image_data).map_err(|e| {
        AppError::Metadata(crate::domain::errors::MetadataError::Validation(format!(
            "Invalid image data: {}",
            e
        )))
    })?;
    let target_path = path.join("preview_custom.png");

    img.save_with_format(&target_path, ImageFormat::Png)
        .map_err(|e| AppError::Io(format!("Failed to save image: {}", e)))?;

    // Invalidate stale cache entries (both image-keyed and folder-keyed)
    // so the next resolve() call re-generates the WebP from the new file.
    ThumbnailCache::invalidate(&target_path);
    ThumbnailCache::invalidate_folder(&path.to_string_lossy());

    // Return the absolute path
    Ok(target_path.to_string_lossy().to_string())
}

#[cfg(test)]
#[path = "tests/mod_thumbnail_cmds_tests.rs"]
mod tests;
