use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::mod_files::metadata;
use crate::services::scanner::core::thumbnail::find_thumbnail;
use std::path::Path;

#[tauri::command]
pub async fn update_mod_thumbnail(
    folder_path: String,
    source_path: String,
) -> Result<String, String> {
    metadata::update_mod_thumbnail(&folder_path, &source_path)
}

#[tauri::command]
pub async fn get_thumbnail(folder_path: String) -> Result<Option<String>, String> {
    let path = Path::new(&folder_path);
    if !path.exists() {
        return Err(format!("Path does not exist: {folder_path}"));
    }

    if let Some(original) = find_thumbnail(path) {
        match ThumbnailCache::get_thumbnail(&original) {
            Ok(cached) => Ok(Some(cached.to_string_lossy().to_string())),
            Err(e) => {
                log::warn!("Thumbnail cache failed: {}", e);
                Ok(Some(original.to_string_lossy().to_string()))
            }
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn paste_thumbnail(folder_path: String, image_data: Vec<u8>) -> Result<String, String> {
    use image::ImageFormat;
    const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

    let path = Path::new(&folder_path);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }

    if image_data.len() > MAX_IMAGE_BYTES {
        return Err("Image too large. Max 10MB.".to_string());
    }

    let img =
        image::load_from_memory(&image_data).map_err(|e| format!("Invalid image data: {}", e))?;
    let target_path = path.join("preview_custom.png");

    img.save_with_format(&target_path, ImageFormat::Png)
        .map_err(|e| format!("Failed to save image: {}", e))?;

    Ok(target_path.to_string_lossy().to_string())
}

#[cfg(test)]
#[path = "tests/mod_thumbnail_cmds_tests.rs"]
mod tests;
