use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::State;

// Re-export from services layer for backward compat (tests use `super::*`)
pub use crate::services::mods::core_ops::{
    rename_mod_folder_inner, standardize_prefix, toggle_mod_inner, RenameResult,
};

#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Open in explorer only supported on Windows".to_string())
}

#[tauri::command]
pub async fn reveal_object_in_explorer(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    object_id: String,
    mods_path: String,
    object_name: String,
) -> Result<String, String> {
    if let Some(path) = resolve_and_heal_db_path(pool.inner(), &object_id).await {
        return open_explorer_select(&path);
    }

    let candidate_path = find_fallback_path(&mods_path, &object_name)?;
    open_explorer_select(&candidate_path)
}

async fn resolve_and_heal_db_path(pool: &sqlx::SqlitePool, object_id: &str) -> Option<String> {
    crate::services::mods::stale_mod_service::resolve_mod_path_for_object(pool, object_id).await
}

fn find_fallback_path(mods_path: &str, object_name: &str) -> Result<String, String> {
    let mods_dir = Path::new(mods_path);
    if !mods_dir.exists() || !mods_dir.is_dir() {
        return Err("Could not find any folder to reveal".to_string());
    }

    let candidate = mods_dir.join(object_name);
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }

    let disabled_candidate = mods_dir.join(format!("{}{}", crate::DISABLED_PREFIX, object_name));
    if disabled_candidate.exists() {
        return Ok(disabled_candidate.to_string_lossy().to_string());
    }

    Ok(mods_path.to_string())
}

fn open_explorer_select(path: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
        Ok(path.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err("Reveal in explorer only supported on Windows".to_string())
}

#[tauri::command]
pub async fn toggle_mod(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    enable: bool,
    game_id: String,
) -> Result<String, String> {
    crate::services::mods::core_ops::toggle_mod_inner_service(
        pool.inner(),
        &state,
        &op_lock,
        path,
        enable,
        &game_id,
    )
    .await
}

#[tauri::command]
pub async fn rename_mod_folder(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    new_name: String,
    game_id: String,
) -> Result<RenameResult, String> {
    crate::services::mods::core_ops::rename_mod_folder_inner_service(
        pool.inner(),
        &state,
        &op_lock,
        folder_path.clone(),
        new_name.clone(),
        &game_id,
    )
    .await
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderContentInfo {
    pub path: String,
    pub name: String,
    pub item_count: usize,
    pub is_empty: bool,
}

pub fn check_folder_contents(path: &Path) -> Result<FolderContentInfo, String> {
    if !path.exists() || !path.is_dir() {
        return Err(format!(
            "Path does not exist or is not a directory: {}",
            path.display()
        ));
    }

    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let item_count = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {e}"))?
        .filter_map(|e| e.ok())
        .count();

    Ok(FolderContentInfo {
        path: path.to_string_lossy().to_string(),
        name,
        item_count,
        is_empty: item_count == 0,
    })
}

#[tauri::command]
pub async fn pre_delete_check(path: String) -> Result<FolderContentInfo, String> {
    check_folder_contents(Path::new(&path))
}

#[cfg(test)]
#[path = "tests/mod_core_cmds_tests.rs"]
mod tests;
