//! Epic 5: Advanced Mod Management Commands
//!
//! Commands that require DB access for "Enable Only This" and conflict checks.
//! Separated from mod_cmds.rs to keep file sizes manageable.

use crate::commands::mods::mod_bulk_cmds::{BulkActionError, BulkResult};
use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
use crate::services::core::operation_lock::OperationLock;
use crate::services::scanner::conflict::{detect_conflicts, ConflictInfo};
use crate::services::scanner::core::walker;
use crate::services::scanner::watcher::WatcherState;
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::State;

/// Info about a duplicate/conflicting enabled mod.
/// Covers: NC-5.2-03
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateInfo {
    pub mod_id: String,
    pub folder_path: String,
    pub actual_name: String,
}

/// Enable a single mod and disable all other mods sharing the same object.
/// Atomic: disable siblings first, then enable target.
///
/// # Covers: TC-5.3-01 (Enable Only This)
#[tauri::command]
pub async fn enable_only_this(
    pool: State<'_, SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    target_path: String,
    game_id: String,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;

    let mut success = Vec::new();
    let mut failures = Vec::new();

    // 1. Find the target mod's object_id from DB
    let target_object_id: Option<String> =
        sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path = ? AND game_id = ?")
            .bind(&target_path)
            .bind(&game_id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

    let object_id = match target_object_id {
        Some(id) => id,
        None => {
            // No object_id — just enable the target without disabling siblings
            let new_path = toggle_mod_inner(&state, target_path, true).await?;
            return Ok(BulkResult {
                success: vec![new_path],
                failures: vec![],
            });
        }
    };

    // 2. Find all other ENABLED mods with the same object_id (siblings)
    let sibling_paths: Vec<String> = sqlx::query_scalar(
        "SELECT folder_path FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 'ENABLED'
         AND folder_path != ?",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind(&target_path)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("DB sibling query failed: {e}"))?;

    // 3. Disable all siblings
    for sibling_path in sibling_paths {
        match toggle_mod_inner(&state, sibling_path.clone(), false).await {
            Ok(new_path) => success.push(new_path),
            Err(e) => failures.push(BulkActionError {
                path: sibling_path,
                error: e,
            }),
        }
    }

    // 4. Enable the target
    match toggle_mod_inner(&state, target_path.clone(), true).await {
        Ok(new_path) => success.push(new_path),
        Err(e) => failures.push(BulkActionError {
            path: target_path,
            error: e,
        }),
    }

    Ok(BulkResult { success, failures })
}

/// Check if enabling a mod would create a duplicate (same object already enabled).
/// Returns list of currently enabled mods sharing the same object_id.
///
/// # Covers: NC-5.2-03 (Duplicate Character Warning)
#[tauri::command]
pub async fn check_duplicate_enabled(
    pool: State<'_, SqlitePool>,
    folder_path: String,
    game_id: String,
) -> Result<Vec<DuplicateInfo>, String> {
    // 1. Get the target mod's object_id
    let target_object_id: Option<String> =
        sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path = ? AND game_id = ?")
            .bind(&folder_path)
            .bind(&game_id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

    let object_id = match target_object_id {
        Some(id) => id,
        None => return Ok(vec![]), // No object — no duplicates possible
    };

    // 2. Find enabled mods with same object
    let duplicates = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, folder_path, actual_name FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 'ENABLED'
         AND folder_path != ?",
    )
    .bind(&object_id)
    .bind(&game_id)
    .bind(&folder_path)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("DB duplicate query failed: {e}"))?;

    Ok(duplicates
        .into_iter()
        .map(|(mod_id, path, name)| DuplicateInfo {
            mod_id,
            folder_path: path,
            actual_name: name,
        })
        .collect())
}

/// Check shader/buffer hash conflicts between a mod and its siblings.
/// Returns conflicts involving the target mod folder.
///
/// # Covers: US-5.7 (Shader Conflict Warning)
#[tauri::command]
pub async fn check_shader_conflicts(folder_path: String) -> Result<Vec<ConflictInfo>, String> {
    let target = PathBuf::from(&folder_path);
    let parent = target.parent().ok_or("Invalid folder path")?;

    // Collect all .ini files from enabled mods in the same directory
    let ini_files = collect_ini_files(parent)?;

    if ini_files.is_empty() {
        return Ok(vec![]);
    }

    // Run conflict detection (CPU-bound, but fast for typical mod counts)
    let conflicts = detect_conflicts(&ini_files);

    // Filter to only conflicts involving the target mod
    let target_str = target.to_string_lossy().to_string();
    let relevant: Vec<ConflictInfo> = conflicts
        .into_iter()
        .filter(|c| c.mod_paths.iter().any(|p| p.starts_with(&target_str)))
        .collect();

    Ok(relevant)
}

/// Walk a directory and collect all `.ini` files from immediate subdirectories.
fn collect_ini_files(mods_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut ini_files = Vec::new();

    let entries =
        std::fs::read_dir(mods_dir).map_err(|e| format!("Failed to read mods dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Skip disabled mods (they won't conflict in-game)
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.to_uppercase().starts_with("DISABLED") {
            continue;
        }
        // Walk this mod folder for .ini files
        walk_ini_recursive(&path, &mut ini_files);
    }

    Ok(ini_files)
}

/// Recursively collect `.ini` files from a directory.
fn walk_ini_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_ini_recursive(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"))
        {
            out.push(path);
        }
    }
}

/// Detect shader/buffer hash conflicts across INI files.
///
/// # Covers: US-2.Z, TC-2.4-01
#[tauri::command]
pub async fn detect_conflicts_cmd(ini_paths: Vec<String>) -> Result<Vec<ConflictInfo>, String> {
    let paths: Vec<PathBuf> = ini_paths.iter().map(PathBuf::from).collect();
    Ok(detect_conflicts(&paths))
}

/// Detect conflicts by scanning the entire mods folder for INI files.
///
/// More efficient for frontend usage as it avoids passing thousands of paths.
/// # Covers: US-2.Z
#[tauri::command]
pub async fn detect_conflicts_in_folder_cmd(
    mods_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    let path = Path::new(&mods_path);
    // Use walker to find all mod folders
    let candidates = walker::scan_mod_folders(path)?;

    let mut all_inis = Vec::new();
    for candidate in candidates {
        // Only check active mods
        if candidate.is_disabled {
            continue;
        }

        let content = walker::scan_folder_content(&candidate.path, 3); // Depth 3 per Epic 2
        all_inis.extend(content.ini_files);
    }

    Ok(detect_conflicts(&all_inis))
}

#[cfg(test)]
#[path = "tests/conflict_cmds_tests.rs"]
mod tests;
