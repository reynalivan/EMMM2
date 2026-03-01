//! Epic 5: Advanced Mod Management Commands
//!
//! Commands that require DB access for "Enable Only This" and conflict checks.
//! Separated from mod_cmds.rs to keep file sizes manageable.

use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::bulk::BulkResult;
use crate::services::scanner::conflict::ConflictInfo;
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

    crate::services::scanner::conflict::enable_only_this_service(
        pool.inner(),
        &state,
        target_path,
        &game_id,
    )
    .await
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
    let dups = crate::services::scanner::conflict::get_duplicates_for_mod_service(
        pool.inner(),
        &folder_path,
        &game_id,
    )
    .await?;

    Ok(dups
        .into_iter()
        .map(|d| DuplicateInfo {
            mod_id: d.mod_id,
            folder_path: d.folder_path,
            actual_name: d.actual_name,
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

    crate::services::scanner::conflict::detect::detect_conflicts_for_mod_service(&target, parent)
}

/// Detect shader/buffer hash conflicts across INI files.
///
/// # Covers: US-2.Z, TC-2.4-01
#[tauri::command]
pub async fn detect_conflicts_cmd(ini_paths: Vec<String>) -> Result<Vec<ConflictInfo>, String> {
    let paths: Vec<(PathBuf, PathBuf)> = ini_paths
        .into_iter()
        .map(|p| {
            let pb = PathBuf::from(p);
            let root = pb
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| pb.clone());
            (root, pb)
        })
        .collect();
    Ok(crate::services::scanner::conflict::detect_conflicts(&paths))
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
    crate::services::scanner::conflict::detect::detect_conflicts_in_folder_service(path)
}

#[cfg(test)]
#[path = "tests/conflict_cmds_tests.rs"]
mod tests;
