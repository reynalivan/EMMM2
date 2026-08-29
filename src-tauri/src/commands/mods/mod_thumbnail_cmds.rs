use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::mods::metadata;
use crate::services::scanner::watcher::WatcherState;

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the thumbnail payload.
pub async fn update_mod_thumbnail(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, OperationLock>,
    game_id: String,
    folder_path: String,
    source_path: String,
) -> Result<String, AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [folder.to_string_lossy().to_string()];
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let guard = watcher.suppressor.suppress_paths([folder.as_ref()]);
    let abs_path = metadata::update_mod_thumbnail(&folder, &source_path)?;
    drop(lock);
    drop(guard);

    // Return the absolute path directly
    Ok(abs_path)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the thumbnail payload.
pub async fn paste_thumbnail(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, OperationLock>,
    game_id: String,
    folder_path: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [folder.to_string_lossy().to_string()];
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let guard = watcher.suppressor.suppress_paths([folder.as_ref()]);
    let saved_path =
        paste_thumbnail_inner(&config, game_id.clone(), folder_path, image_data).await?;
    drop(lock);
    drop(guard);
    Ok(saved_path)
}

pub async fn paste_thumbnail_inner(
    config: &ConfigService,
    game_id: String,
    folder_path: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    use image::ImageFormat;
    use std::io::Cursor;

    crate::services::mods::preview_ops::ensure_image_size(&image_data)?;

    let path = validate_path(config, &game_id, &folder_path)?;

    let img = image::load_from_memory(&image_data).map_err(|e| {
        AppError::Metadata(crate::domain::errors::MetadataError::Validation(format!(
            "Invalid image data: {}",
            e
        )))
    })?;
    let target_path = path.join("preview_custom.png");

    let mut encoded = Vec::new();
    img.write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
        .map_err(|e| AppError::Io(format!("Failed to encode image: {e}")))?;
    crate::services::fs_utils::atomic_file::atomic_write(&target_path, &encoded)?;

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
