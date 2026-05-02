//! Soft delete (Trash) service for mod folders.
//!
//! Moves mod folders to `./app_data/trash/{uuid}/` with metadata JSON for restore.
//! Does NOT use the OS Recycle Bin — uses a custom app-level trash.
//!
//! # Covers: US-4.4 (Soft Delete), TC-4.5-01, DI-4.01

use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use uuid::Uuid;

/// Metadata stored alongside each trashed item for restore.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TrashMetadata {
    /// Unique ID for this trash entry
    pub id: String,
    /// Original absolute path before deletion
    pub original_path: String,
    /// Display name of the mod folder
    pub original_name: String,
    /// ISO 8601 timestamp of deletion
    pub deleted_at: String,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Associated game_id (for DB cleanup)
    pub game_id: Option<String>,
}

/// Move a mod folder to the trash directory.
///
/// Creates `{trash_dir}/{uuid}/` containing:
/// - The original folder contents
/// - `metadata.json` with restore information
///
/// Returns the `TrashMetadata` on success.
pub async fn delete_mod_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    trash_dir: std::path::PathBuf,
    path: String,
    game_id: Option<String>,
) -> Result<(), AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Io)?;

    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir)
            .map_err(|e| AppError::Io(format!("Failed to create trash dir: {}", e)))?;
    }

    if let Some(ref gid) = game_id {
        PathGuard::validate_path(config, gid, &path).map_err(AppError::Security)?;
    }

    let (is_safe, object_id) = if let Some(ref gid) = game_id {
        let mods_path = crate::repo::game_repo::get_mod_path(pool, gid)
            .await
            .ok()
            .flatten();

        if let Some(mp) = mods_path {
            let base = Path::new(&mp);
            let rel = Path::new(&path)
                .strip_prefix(base)
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());

            let safe = sqlx::query_scalar::<_, i32>(
                "SELECT is_safe FROM mods WHERE game_id = ? AND folder_path = ? LIMIT 1",
            )
            .bind(gid)
            .bind(&rel)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|value| value != 0);
            let object = crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, &rel, gid)
                .await
                .ok()
                .flatten();
            (safe, object)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let path_obj = Path::new(&path);
    let _guard = SuppressionGuard::new(&state.suppressor);

    move_to_trash(path_obj, &trash_dir, game_id.clone())?;
    let _ = crate::repo::mod_repo::delete_mod_by_path(pool, &path).await;

    if let (Some(gid), Some(safe)) = (game_id, is_safe) {
        let changed_object_ids = object_id.into_iter().collect::<Vec<_>>();
        let _ = crate::services::runtime_projection_service::refresh_projection_for_object_ids(
            pool,
            &gid,
            &changed_object_ids,
            false,
        )
        .await;
        let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
            pool,
            config,
            state.suppressor.clone(),
            &gid,
            &[safe],
            true,
            true,
        )
        .await;
    }

    Ok(())
}

/// Helper that suppresses the watcher for the single move action.
pub async fn move_to_trash_guarded(
    state: &WatcherState,
    trash_dir: &Path,
    path: String,
    game_id: Option<String>,
) -> Result<(), AppError> {
    let path_obj = Path::new(&path);
    let _guard = SuppressionGuard::new(&state.suppressor);
    move_to_trash(path_obj, trash_dir, game_id).map(|_| ())
}

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
                .and_then(|_| {
                    fs::remove_dir_all(source_path)
                        .map_err(|e| format!("Failed to remove source after copy: {e}"))
                })
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

/// Format epoch seconds as ISO-8601 string (basic, no chrono dependency).
fn chrono_format_epoch(secs: u64) -> String {
    // Simple UTC format: YYYY-MM-DDTHH:MM:SSZ
    // We use a basic calculation since we don't have chrono
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to Y/M/D (simplified Gregorian)
    let (year, month, day) = epoch_days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn epoch_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's date library
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_adj = if m <= 2 { y + 1 } else { y };
    (y_adj, m, d)
}

/// Recursively copy a directory (fallback for cross-device moves).
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("mkdir failed: {e}"))?;

    for entry in fs::read_dir(src).map_err(|e| format!("read_dir failed: {e}"))? {
        let entry = entry.map_err(|e| format!("entry error: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy failed: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/trash_tests.rs"]
mod tests;
