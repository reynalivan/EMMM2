//! Epic 5: Advanced Mod Management Commands
//!
//! Commands that require DB access for "Enable Only This" and conflict checks.
//! Separated from mod_cmds.rs to keep file sizes manageable.

use crate::domain::errors::AppError;
use crate::services::scanner::conflict::ConflictInfo;
use std::path::{Path, PathBuf};

/// Detect shader/buffer hash conflicts across INI files.
///
/// # Covers: US-2.Z, TC-2.4-01
#[specta::specta]
#[tauri::command]
pub async fn detect_conflicts_cmd(ini_paths: Vec<String>) -> Result<Vec<ConflictInfo>, AppError> {
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
#[specta::specta]
#[tauri::command]
pub async fn detect_conflicts_in_folder_cmd(
    mods_path: String,
) -> Result<Vec<ConflictInfo>, AppError> {
    let path = Path::new(&mods_path);
    crate::services::scanner::conflict::detect::detect_conflicts_in_folder_service(path)
        .map_err(AppError::Internal)
}

#[cfg(test)]
#[path = "tests/conflict_cmds_tests.rs"]
mod tests;
