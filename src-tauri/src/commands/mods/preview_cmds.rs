use crate::domain::errors::{AppError, MetadataError};
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::ini::document::IniDocument;
use crate::services::mods::preview_ops::{
    clear_mod_preview_images_inner, list_mod_ini_files_inner, list_mod_preview_images_inner,
    read_mod_ini_inner, remove_mod_preview_image_inner, resolve_image_path,
    save_mod_preview_image_inner, write_mod_ini_locked_inner,
};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use tauri::State;

pub use crate::services::mods::preview_ops::{IniFileEntry, IniLineUpdate};

#[specta::specta]
#[tauri::command]
pub async fn list_mod_ini_files(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<IniFileEntry>, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    list_mod_ini_files_inner(&mod_root)
}

#[specta::specta]
#[tauri::command]
pub async fn read_mod_ini(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
    file_name: String,
) -> Result<IniDocument, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    read_mod_ini_inner(&mod_root, &file_name)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn write_mod_ini(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    file_name: String,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let changed_path = mod_root.join(&file_name).to_string_lossy().to_string();
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    write_mod_ini_locked_inner(&op_lock, &mod_root, &file_name, line_updates).await?;
    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, vec![changed_path])
        .await
        .map_err(AppError::Internal)
}

#[specta::specta]
#[tauri::command]
pub async fn list_mod_preview_images(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<String>, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    list_mod_preview_images_inner(&mod_root)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn save_mod_preview_image(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    object_name: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

    if image_data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Metadata(MetadataError::Validation(
            "Image too large. Max 10MB.".to_string(),
        )));
    }

    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let saved = save_mod_preview_image_inner(&mod_root, &object_name, &image_data)?;
    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, vec![saved.clone()])
        .await
        .map_err(AppError::Internal)?;
    Ok(saved)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn remove_mod_preview_image(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    image_path: String,
) -> Result<(), AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let target = resolve_image_path(&mod_root, &image_path)?;
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    remove_mod_preview_image_inner(&mod_root, &image_path)?;
    emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
        vec![target.to_string_lossy().to_string()],
    )
    .await
    .map_err(AppError::Internal)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn clear_mod_preview_images(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<String>, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let removed = clear_mod_preview_images_inner(&mod_root)?;
    let changed_paths = if removed.is_empty() {
        vec![mod_root.to_string_lossy().to_string()]
    } else {
        removed.clone()
    };
    emit_internal_disk_reconcile(&app, pool.inner(), &game_id, changed_paths)
        .await
        .map_err(AppError::Internal)?;
    Ok(removed)
}
