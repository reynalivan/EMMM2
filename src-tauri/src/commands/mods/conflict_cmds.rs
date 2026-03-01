use std::fs;
use std::path::Path;

use tauri::State;

use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

/// Resolve a naming conflict where both "X" and "DISABLED X" exist on disk.
///
/// Strategies:
/// - `keep_enabled`: Keep the enabled folder, rename the disabled duplicate
/// - `keep_disabled`: Keep the disabled folder, rename the enabled duplicate
/// - `separate`: Rename one folder's base name to make them unique
#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    keep_path: String,
    duplicate_path: String,
    strategy: String,
) -> Result<String, String> {
    let _lock = op_lock.acquire().await?;
    resolve_conflict_inner(&state, &keep_path, &duplicate_path, &strategy)
}

pub fn resolve_conflict_inner(
    state: &WatcherState,
    keep_path: &str,
    duplicate_path: &str,
    strategy: &str,
) -> Result<String, String> {
    let keep = Path::new(keep_path);
    let dup = Path::new(duplicate_path);

    if !keep.exists() {
        return Err(format!("Keep path does not exist: {keep_path}"));
    }
    if !dup.exists() {
        return Err(format!("Duplicate path does not exist: {duplicate_path}"));
    }

    let parent = dup.parent().unwrap_or_else(|| Path::new(""));
    let dup_name = dup.file_name().unwrap_or_default().to_string_lossy();

    let new_name = match strategy {
        "keep_enabled" | "keep_disabled" => {
            // Rename the duplicate with a "(dup)" suffix to break the collision.
            let base = normalize_display_name(&dup_name);
            let is_disabled = is_disabled_folder(&dup_name);

            find_unique_name(parent, &base, is_disabled)
        }
        "separate" => {
            // Rename the duplicate's base name to "<base> (copy)", keeping its prefix status.
            let base = normalize_display_name(&dup_name);
            let is_disabled = is_disabled_folder(&dup_name);
            let copy_base = format!("{} (copy)", base);
            find_unique_name(parent, &copy_base, is_disabled)
        }
        _ => return Err(format!("Unknown strategy: {strategy}")),
    };

    let new_path = parent.join(&new_name);

    // Final guard: new target must not exist
    if new_path.exists() {
        return Err(format!("Target already exists: {}", new_path.display()));
    }

    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        fs::rename(dup, &new_path).map_err(|e| format!("Failed to rename duplicate: {e}"))?;
    }

    log::info!(
        "Resolved conflict: '{}' → '{}'",
        dup_name,
        new_path.display()
    );

    Ok(new_path.to_string_lossy().to_string())
}

/// Find a unique folder name by appending "(dup N)" if needed.
/// Returns the full folder name (with DISABLED prefix if `is_disabled` is true).
fn find_unique_name(parent: &Path, base: &str, is_disabled: bool) -> String {
    let prefix = if is_disabled {
        crate::DISABLED_PREFIX
    } else {
        ""
    };

    // Try without suffix first
    let candidate = format!("{}{} (dup)", prefix, base);
    if !parent.join(&candidate).exists() {
        return candidate;
    }

    // Try with incrementing number
    for n in 2..100 {
        let candidate = format!("{}{} (dup {})", prefix, base, n);
        if !parent.join(&candidate).exists() {
            return candidate;
        }
    }

    // Fallback: use timestamp
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}{} (dup {})", prefix, base, ts)
}

// ── Conflict Details (for comparison dialog) ─────────────────────────────────

#[derive(serde::Serialize)]
pub struct FolderDetail {
    pub path: String,
    pub folder_name: String,
    pub is_enabled: bool,
    pub total_size: u64,
    pub file_count: usize,
    pub files: Vec<FileEntry>,
    pub thumbnail_path: Option<String>,
}

#[derive(serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub is_ini: bool,
}

#[derive(serde::Serialize)]
pub struct ConflictDetails {
    pub enabled: FolderDetail,
    pub disabled: FolderDetail,
}

/// Get detailed file listings for both enabled and disabled versions of a conflicting folder.
/// Used by the enhanced ConflictResolveDialog for side-by-side comparison.
#[tauri::command]
pub async fn get_conflict_details(
    enabled_path: String,
    disabled_path: String,
) -> Result<ConflictDetails, String> {
    let enabled = scan_folder_detail(&enabled_path, true)?;
    let disabled = scan_folder_detail(&disabled_path, false)?;
    Ok(ConflictDetails { enabled, disabled })
}

fn scan_folder_detail(path_str: &str, is_enabled: bool) -> Result<FolderDetail, String> {
    let path = Path::new(path_str);
    if !path.exists() || !path.is_dir() {
        return Err(format!(
            "Path does not exist or is not a directory: {path_str}"
        ));
    }

    let folder_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut files = Vec::new();
    let mut total_size: u64 = 0;
    let mut thumbnail_path: Option<String> = None;

    // Scan top-level files (non-recursive for performance)
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let lower = name.to_lowercase();
                let is_ini = lower.ends_with(".ini");

                // Detect thumbnail
                if thumbnail_path.is_none() {
                    let thumb_exts = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
                    if thumb_exts.iter().any(|ext| lower.ends_with(ext)) {
                        // Prefer "preview" or "thumbnail" named images
                        let stem = lower.rsplit('.').nth(1).unwrap_or("");
                        if stem.contains("preview")
                            || stem.contains("thumb")
                            || stem.contains("icon")
                        {
                            thumbnail_path = Some(entry_path.to_string_lossy().to_string());
                        }
                    }
                }

                total_size += size;
                files.push(FileEntry { name, size, is_ini });
            } else if entry_path.is_dir() {
                // Count subdir size (shallow)
                if let Ok(sub_entries) = fs::read_dir(&entry_path) {
                    for se in sub_entries.flatten() {
                        if se.path().is_file() {
                            total_size += se.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                    }
                }
            }
        }
    }

    // If no priority thumbnail found, take the first image file
    if thumbnail_path.is_none() {
        for f in &files {
            let lower = f.name.to_lowercase();
            let thumb_exts = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
            if thumb_exts.iter().any(|ext| lower.ends_with(ext)) {
                thumbnail_path = Some(path.join(&f.name).to_string_lossy().to_string());
                break;
            }
        }
    }

    // Sort files: INI first, then by name
    files.sort_by(|a, b| b.is_ini.cmp(&a.is_ini).then(a.name.cmp(&b.name)));

    Ok(FolderDetail {
        path: path_str.to_string(),
        folder_name: folder_name.clone(),
        is_enabled,
        total_size,
        file_count: files.len(),
        files,
        thumbnail_path,
    })
}

#[cfg(test)]
#[path = "tests/conflict_cmds_tests.rs"]
mod tests;
