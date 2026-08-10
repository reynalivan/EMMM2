//! Compatibility layer for legacy trash commands.

/// Resolve the app-level trash directory.
///
/// The `{app_data}/trash` layout belongs to this module; nine call sites used
/// to re-derive it, each re-spelling the "failed to get app data dir" error.
pub fn trash_dir(
    app: &tauri::AppHandle,
) -> Result<std::path::PathBuf, crate::domain::errors::AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        crate::domain::errors::AppError::Io(format!("Failed to get app data dir: {error}"))
    })?;
    Ok(trash_dir_under(&app_data_dir))
}

/// Same layout, for callers that already hold the app data directory.
pub fn trash_dir_under(app_data_dir: &std::path::Path) -> std::path::PathBuf {
    app_data_dir.join(TRASH_DIR_NAME)
}

/// Directory name the trash lives under, inside the app data directory.
const TRASH_DIR_NAME: &str = "trash";

use super::timestamp::chrono_format_epoch;
use super::types::TrashMetadata;
use crate::domain::errors::AppError;
use std::path::Path;
use std::time::SystemTime;
use uuid::Uuid;

pub fn move_to_trash(
    source_path: &Path,
    trash_dir: &Path,
    game_id: Option<String>,
) -> Result<TrashMetadata, AppError> {
    if !source_path.exists() {
        return Err(AppError::Io(format!(
            "Source does not exist: {}",
            source_path.display()
        )));
    }
    if !source_path.is_dir() {
        return Err(AppError::Io("Only directories can be trashed".to_string()));
    }

    let folder_name = source_path
        .file_name()
        .ok_or_else(|| AppError::Internal("Invalid folder name".to_string()))?
        .to_string_lossy()
        .to_string();

    let _ = trash_dir;
    let trash_id = Uuid::new_v4().to_string();
    let size_bytes = 0;

    // Build metadata
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let deleted_at = chrono_format_epoch(now.as_secs()).to_string();

    let metadata = TrashMetadata {
        id: trash_id.clone(),
        original_path: source_path.to_string_lossy().to_string(),
        original_name: folder_name.clone(),
        deleted_at,
        size_bytes,
        game_id,
    };

    crate::services::fs_utils::recycle_bin::move_path_to_recycle_bin(source_path)?;

    log::info!("Moved '{}' to trash (id: {})", folder_name, trash_id);
    Ok(metadata)
}

/// Restore a trashed item back to its original location.
pub fn restore_from_trash(
    trash_id: &str,
    trash_dir: &Path,
    target_game_id: Option<&String>,
) -> Result<String, AppError> {
    let _ = (trash_id, trash_dir, target_game_id);
    Err(AppError::Validation(
        "Restore deleted items from the system Recycle Bin.".to_string(),
    ))
}

/// List all items in the trash directory.
pub fn list_trash(trash_dir: &Path) -> Result<Vec<TrashMetadata>, AppError> {
    let _ = trash_dir;
    Ok(Vec::new())
}

/// Permanently delete all items in the trash.
pub fn empty_trash(trash_dir: &Path) -> Result<u64, AppError> {
    let _ = trash_dir;
    Err(AppError::Validation(
        "The system Recycle Bin is managed by the operating system.".to_string(),
    ))
}
