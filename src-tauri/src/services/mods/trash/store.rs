//! Filesystem-level trash store: move in, restore out, list and empty.

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
use std::fs;
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

    let trash_id = Uuid::new_v4().to_string();
    let trash_entry_dir = trash_dir.join(&trash_id);

    // Create trash entry directory
    fs::create_dir_all(&trash_entry_dir)
        .map_err(|e| AppError::Io(format!("Failed to create trash entry: {e}")))?;

    // Calculate size before move (shallow)
    let size_bytes = fs::metadata(source_path).map(|m| m.len()).unwrap_or(0);

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

    // Write metadata.json first (before moving content)
    let metadata_path = trash_entry_dir.join("metadata.json");
    let json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| AppError::Io(format!("Failed to serialize metadata: {e}")))?;
    fs::write(&metadata_path, json)
        .map_err(|e| AppError::Io(format!("Failed to write metadata: {e}")))?;

    // Move the folder content into trash entry
    let dest = trash_entry_dir.join(&folder_name);
    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(source_path, &dest)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let processes =
                    crate::services::fs_utils::locking::get_locking_processes(source_path);
                if !processes.is_empty() {
                    return AppError::FileInUse {
                        path: source_path.to_string_lossy().to_string(),
                        processes,
                    };
                }
            }
            // If rename fails (cross-device), try copy + delete
            log::warn!("rename failed, attempting copy: {e}");
            copy_dir_recursive(source_path, &dest)
                .and_then(|_| Ok(fs::remove_dir_all(source_path)?))
                .unwrap_or_else(|copy_err| {
                    log::error!("Copy fallback also failed: {copy_err}");
                });
            AppError::Io(format!("Failed to move to trash: {e}"))
        })?;

    log::info!("Moved '{}' to trash (id: {})", folder_name, trash_id);
    Ok(metadata)
}

/// Restore a trashed item back to its original location.
pub fn restore_from_trash(
    trash_id: &str,
    trash_dir: &Path,
    target_game_id: Option<&String>,
) -> Result<String, AppError> {
    let entry_dir = trash_dir.join(trash_id);
    if !entry_dir.exists() {
        return Err(AppError::Io(format!("Trash entry not found: {trash_id}")));
    }

    // Read metadata
    let metadata_path = entry_dir.join("metadata.json");
    let raw = fs::read_to_string(&metadata_path)
        .map_err(|e| AppError::Io(format!("Failed to read trash metadata: {e}")))?;
    let metadata: TrashMetadata = serde_json::from_str(&raw)
        .map_err(|e| AppError::Io(format!("Invalid trash metadata: {e}")))?;

    // Context Parity Check: Prevent restoring a mod into the wrong game context
    if let (Some(meta_game), Some(target_game)) = (&metadata.game_id, target_game_id) {
        if meta_game != target_game {
            return Err(AppError::Io(
                "Context mismatch: Cannot restore a mod from a different game".to_string(),
            ));
        }
    } else if metadata.game_id.is_some() && target_game_id.is_none() {
        return Err(AppError::Io(
            "Context mismatch: Target game context is missing".to_string(),
        ));
    }

    let original = Path::new(&metadata.original_path);

    // Check if original location already exists
    if original.exists() {
        return Err(AppError::Io(format!(
            "Original path already exists: {}",
            original.display()
        )));
    }

    // Find the content folder (should be the only non-metadata item)
    let content_dir = entry_dir.join(&metadata.original_name);
    if !content_dir.exists() {
        return Err(AppError::Io(
            "Trash content missing — cannot restore".to_string(),
        ));
    }

    // Move back
    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(&content_dir, original)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let processes =
                    crate::services::fs_utils::locking::get_locking_processes(&content_dir);
                if !processes.is_empty() {
                    return AppError::FileInUse {
                        path: content_dir.to_string_lossy().to_string(),
                        processes,
                    };
                }
            }
            AppError::Io(format!("Failed to restore from trash: {e}"))
        })?;

    // Cleanup trash entry
    fs::remove_dir_all(&entry_dir)
        .map_err(|e| AppError::Io(format!("Failed to cleanup trash entry: {e}")))?;

    log::info!("Restored '{}' from trash", metadata.original_name);
    Ok(metadata.original_path)
}

/// List all items in the trash directory.
pub fn list_trash(trash_dir: &Path) -> Result<Vec<TrashMetadata>, AppError> {
    if !trash_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(trash_dir)
        .map_err(|e| AppError::Io(format!("Failed to read trash dir: {e}")))?;

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let meta_path = entry.path().join("metadata.json");
        if meta_path.exists() {
            if let Ok(raw) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<TrashMetadata>(&raw) {
                    items.push(meta);
                }
            }
        }
    }

    // Sort by deleted_at descending (newest first)
    items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    Ok(items)
}

/// Permanently delete all items in the trash.
pub fn empty_trash(trash_dir: &Path) -> Result<u64, AppError> {
    if !trash_dir.exists() {
        return Ok(0);
    }

    let entries =
        fs::read_dir(trash_dir).map_err(|e| AppError::Io(format!("Failed to read trash: {e}")))?;

    let mut count = 0u64;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            fs::remove_dir_all(entry.path())
                .map_err(|e| AppError::Io(format!("Failed to remove trash entry: {e}")))?;
            count += 1;
        }
    }

    log::info!("Emptied trash: {} entries removed", count);
    Ok(count)
}

/// Recursively copy a directory (fallback for cross-device moves).
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
