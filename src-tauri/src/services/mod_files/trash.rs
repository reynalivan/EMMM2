//! Soft delete (Trash) service for mod folders.
//!
//! Moves mod folders to `./app_data/trash/{uuid}/` with metadata JSON for restore.
//! Does NOT use the OS Recycle Bin — uses a custom app-level trash.
//!
//! # Covers: US-4.4 (Soft Delete), TC-4.5-01, DI-4.01

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use uuid::Uuid;

/// Metadata stored alongside each trashed item for restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub fn move_to_trash(
    source_path: &Path,
    trash_dir: &Path,
    game_id: Option<String>,
) -> Result<TrashMetadata, String> {
    if !source_path.exists() {
        return Err(format!("Source does not exist: {}", source_path.display()));
    }
    if !source_path.is_dir() {
        return Err("Only directories can be trashed".to_string());
    }

    let folder_name = source_path
        .file_name()
        .ok_or("Invalid folder name")?
        .to_string_lossy()
        .to_string();

    let trash_id = Uuid::new_v4().to_string();
    let trash_entry_dir = trash_dir.join(&trash_id);

    // Create trash entry directory
    fs::create_dir_all(&trash_entry_dir)
        .map_err(|e| format!("Failed to create trash entry: {e}"))?;

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
        .map_err(|e| format!("Failed to serialize metadata: {e}"))?;
    fs::write(&metadata_path, json).map_err(|e| format!("Failed to write metadata: {e}"))?;

    // Move the folder content into trash entry
    let dest = trash_entry_dir.join(&folder_name);
    fs::rename(source_path, &dest).map_err(|e| {
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
        format!("Failed to move to trash: {e}")
    })?;

    log::info!("Moved '{}' to trash (id: {})", folder_name, trash_id);
    Ok(metadata)
}

/// Restore a trashed item back to its original location.
pub fn restore_from_trash(trash_id: &str, trash_dir: &Path) -> Result<String, String> {
    let entry_dir = trash_dir.join(trash_id);
    if !entry_dir.exists() {
        return Err(format!("Trash entry not found: {trash_id}"));
    }

    // Read metadata
    let metadata_path = entry_dir.join("metadata.json");
    let raw = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Failed to read trash metadata: {e}"))?;
    let metadata: TrashMetadata =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid trash metadata: {e}"))?;

    let original = Path::new(&metadata.original_path);

    // Check if original location already exists
    if original.exists() {
        return Err(format!(
            "Original path already exists: {}",
            original.display()
        ));
    }

    // Find the content folder (should be the only non-metadata item)
    let content_dir = entry_dir.join(&metadata.original_name);
    if !content_dir.exists() {
        return Err("Trash content missing — cannot restore".to_string());
    }

    // Move back
    fs::rename(&content_dir, original).map_err(|e| format!("Failed to restore from trash: {e}"))?;

    // Cleanup trash entry
    fs::remove_dir_all(&entry_dir).map_err(|e| format!("Failed to cleanup trash entry: {e}"))?;

    log::info!("Restored '{}' from trash", metadata.original_name);
    Ok(metadata.original_path)
}

/// List all items in the trash directory.
pub fn list_trash(trash_dir: &Path) -> Result<Vec<TrashMetadata>, String> {
    if !trash_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(trash_dir).map_err(|e| format!("Failed to read trash dir: {e}"))?;

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
pub fn empty_trash(trash_dir: &Path) -> Result<u64, String> {
    if !trash_dir.exists() {
        return Ok(0);
    }

    let entries = fs::read_dir(trash_dir).map_err(|e| format!("Failed to read trash: {e}"))?;

    let mut count = 0u64;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            fs::remove_dir_all(entry.path())
                .map_err(|e| format!("Failed to remove trash entry: {e}"))?;
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
