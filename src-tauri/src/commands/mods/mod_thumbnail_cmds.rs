use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::mods::metadata;
use crate::services::scanner::core::thumbnail::find_thumbnail;

#[specta::specta]
#[tauri::command]
pub async fn update_mod_thumbnail(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
    source_path: String,
) -> Result<String, AppError> {
    let abs_path = metadata::update_mod_thumbnail(&config, &game_id, &folder_path, &source_path)?;

    // Return the absolute path directly
    Ok(abs_path)
}

#[specta::specta]
#[tauri::command]
pub async fn get_thumbnail(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Option<String>, AppError> {
    let path = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(crate::domain::errors::MetadataError::Security(e)))?;

    if let Some(original) = find_thumbnail(&path) {
        match ThumbnailCache::get_thumbnail(&game_id, &original) {
            Ok(path) => Ok(Some(path)), // Now returns absolute path

            Err(e) => {
                log::warn!("Thumbnail cache failed, using un-resized asset: {}", e);
                Ok(Some(original.to_string_lossy().to_string()))
            }
        }
    } else {
        Ok(None)
    }
}

#[specta::specta]
#[tauri::command]
pub async fn paste_thumbnail(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    paste_thumbnail_inner(&config, game_id, folder_path, image_data).await
}

pub async fn paste_thumbnail_inner(
    config: &ConfigService,
    game_id: String,
    folder_path: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    use image::ImageFormat;
    const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

    let path = PathGuard::validate_path(config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(crate::domain::errors::MetadataError::Security(e)))?;

    if image_data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Metadata(
            crate::domain::errors::MetadataError::Validation(
                "Image too large. Max 10MB.".to_string(),
            ),
        ));
    }

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
