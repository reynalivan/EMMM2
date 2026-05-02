use crate::domain::errors::{AppError, MetadataError};
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::orchestrator::DiskReconcileState;
use crate::services::disk_reconcile::types::DiskReconcileReason;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::ini::document::{self as ini_document, IniDocument};
use crate::services::ini::write as ini_write;
use crate::services::mods::preview_image;
use crate::services::scanner::core::thumbnail;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use tauri::{Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
pub struct IniFileEntry {
    pub filename: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
pub struct IniLineUpdate {
    pub line_idx: usize,
    pub content: String,
}

fn validate_ini_filename(file_name: &str) -> Result<(), AppError> {
    if file_name.trim().is_empty() {
        return Err(AppError::Metadata(MetadataError::Validation(
            "INI filename cannot be empty".to_string(),
        )));
    }

    let name_path = Path::new(file_name);
    if name_path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::CurDir | Component::RootDir
        )
    }) {
        return Err(AppError::Metadata(MetadataError::Security(
            "Invalid INI filename path".to_string(),
        )));
    }

    if name_path.components().count() != 1 {
        return Err(AppError::Metadata(MetadataError::Validation(
            "INI filename must not include directories".to_string(),
        )));
    }

    let lower = file_name.to_ascii_lowercase();
    if lower == "desktop.ini" {
        return Err(AppError::Metadata(MetadataError::Validation(
            "desktop.ini is not a valid editable INI".to_string(),
        )));
    }
    if !lower.ends_with(".ini") {
        return Err(AppError::Metadata(MetadataError::Validation(
            "Only .ini files are supported".to_string(),
        )));
    }

    Ok(())
}

fn resolve_ini_path(mod_root: &Path, file_name: &str) -> Result<PathBuf, AppError> {
    validate_ini_filename(file_name)?;

    let target = mod_root.join(file_name);
    // PathGuard already canonicalizes the root, so we just need to ensure the target is within it.
    // However, for extra safety we check if it starts with mod_root.
    if !target.starts_with(mod_root) {
        return Err(AppError::Metadata(MetadataError::Security(
            "INI file path escapes mod folder".to_string(),
        )));
    }

    if !target.is_file() {
        return Err(AppError::Metadata(MetadataError::NotFound(format!(
            "INI file not found: {}",
            target.display()
        ))));
    }

    Ok(target)
}

fn resolve_image_path(mod_root: &Path, image_path: &str) -> Result<PathBuf, AppError> {
    if image_path.trim().is_empty() {
        return Err(AppError::Metadata(MetadataError::Validation(
            "Image path cannot be empty".to_string(),
        )));
    }

    let raw = Path::new(image_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        mod_root.join(raw)
    };

    // Use canonicalize to resolve symlinks and '..' if any (though PathGuard should prevent escaping)
    let canonical_target = candidate.canonicalize().map_err(|e| {
        AppError::Metadata(MetadataError::NotFound(format!(
            "Failed to resolve image path: {e}"
        )))
    })?;

    if !canonical_target.starts_with(mod_root) {
        return Err(AppError::Metadata(MetadataError::Security(
            "Image path escapes mod folder".to_string(),
        )));
    }

    Ok(canonical_target)
}

pub fn list_mod_ini_files_inner(mod_root: &Path) -> Result<Vec<IniFileEntry>, AppError> {
    let files = ini_document::list_ini_files(mod_root).map_err(AppError::Io)?;
    Ok(files
        .into_iter()
        .map(|path| IniFileEntry {
            filename: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: path.to_string_lossy().to_string(),
        })
        .collect())
}

pub fn read_mod_ini_inner(mod_root: &Path, file_name: &str) -> Result<IniDocument, AppError> {
    let ini_path = resolve_ini_path(mod_root, file_name)?;
    ini_document::read_ini_document(&ini_path).map_err(AppError::Io)
}

pub fn write_mod_ini_inner(
    mod_root: &Path,
    file_name: &str,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), AppError> {
    let ini_path = resolve_ini_path(mod_root, file_name)?;
    let document = ini_document::read_ini_document(&ini_path).map_err(AppError::Io)?;
    let updates: Vec<(usize, String)> = line_updates
        .into_iter()
        .map(|u| (u.line_idx, u.content))
        .collect();

    ini_write::save_ini_with_updates(&document, &updates).map_err(AppError::Io)
}

pub async fn write_mod_ini_locked_inner(
    op_lock: &OperationLock,
    mod_root: &Path,
    file_name: &str,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    write_mod_ini_inner(mod_root, file_name, line_updates)
}

pub fn list_mod_preview_images_inner(mod_root: &Path) -> Result<Vec<String>, AppError> {
    if !mod_root.exists() || !mod_root.is_dir() {
        return Err(AppError::Metadata(MetadataError::Validation(format!(
            "Invalid mod folder: {}",
            mod_root.display()
        ))));
    }

    Ok(thumbnail::list_preview_images(mod_root)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

pub fn save_mod_preview_image_inner(
    mod_root: &Path,
    object_name: &str,
    image_data: &[u8],
) -> Result<String, AppError> {
    let saved = preview_image::save_preview_image(mod_root, object_name, image_data)
        .map_err(AppError::Io)?;
    Ok(saved.to_string_lossy().to_string())
}

pub fn remove_mod_preview_image_inner(mod_root: &Path, image_path: &str) -> Result<(), AppError> {
    let target = resolve_image_path(mod_root, image_path)?;
    preview_image::remove_preview_image(mod_root, &target).map_err(AppError::Io)
}

pub fn clear_mod_preview_images_inner(mod_root: &Path) -> Result<Vec<String>, AppError> {
    preview_image::clear_preview_images(mod_root).map_err(AppError::Io)
}

async fn emit_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &ConfigService,
    disk_reconcile_state: &DiskReconcileState,
    watcher: &WatcherState,
    game_id: &str,
    changed_paths: Vec<String>,
) -> Result<(), AppError> {
    let result = crate::services::disk_reconcile::orchestrator::reconcile_disk_state(
        app,
        pool,
        config,
        disk_reconcile_state,
        watcher.suppressor.clone(),
        game_id.to_string(),
        DiskReconcileReason::InternalMutation,
        changed_paths,
        false,
    )
    .await
    .map_err(AppError::Internal)?;

    app.emit("disk_reconcile:result", result)
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[specta::specta]
#[tauri::command]
pub async fn list_mod_ini_files(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<IniFileEntry>, AppError> {
    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
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
    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    read_mod_ini_inner(&mod_root, &file_name)
}

#[specta::specta]
#[tauri::command]
pub async fn write_mod_ini(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<'_, DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    file_name: String,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), AppError> {
    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let changed_path = mod_root.join(&file_name).to_string_lossy().to_string();
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    write_mod_ini_locked_inner(&op_lock, &mod_root, &file_name, line_updates).await?;
    emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        &watcher,
        &game_id,
        vec![changed_path],
    )
    .await
}

#[specta::specta]
#[tauri::command]
pub async fn list_mod_preview_images(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<String>, AppError> {
    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    list_mod_preview_images_inner(&mod_root)
}

#[specta::specta]
#[tauri::command]
pub async fn save_mod_preview_image(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<'_, DiskReconcileState>,
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

    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let saved = save_mod_preview_image_inner(&mod_root, &object_name, &image_data)?;
    emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        &watcher,
        &game_id,
        vec![saved.clone()],
    )
    .await?;
    Ok(saved)
}

#[specta::specta]
#[tauri::command]
pub async fn remove_mod_preview_image(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<'_, DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    image_path: String,
) -> Result<(), AppError> {
    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let target = resolve_image_path(&mod_root, &image_path)?;
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    remove_mod_preview_image_inner(&mod_root, &image_path)?;
    emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        &watcher,
        &game_id,
        vec![target.to_string_lossy().to_string()],
    )
    .await
}

#[specta::specta]
#[tauri::command]
pub async fn clear_mod_preview_images(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<'_, DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<String>, AppError> {
    let mod_root = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(MetadataError::Security(e)))?;
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let removed = clear_mod_preview_images_inner(&mod_root)?;
    let changed_paths = if removed.is_empty() {
        vec![mod_root.to_string_lossy().to_string()]
    } else {
        removed.clone()
    };
    emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        &watcher,
        &game_id,
        changed_paths,
    )
    .await?;
    Ok(removed)
}

#[cfg(test)]
#[path = "tests/preview_cmds_tests.rs"]
mod tests;
