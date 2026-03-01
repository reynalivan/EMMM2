use crate::services::fs_utils::operation_lock::OperationLock;

use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use tauri::State;

static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").unwrap());

pub fn standardize_prefix(folder_name: &str, target_enabled: bool) -> String {
    let clean_name = DISABLED_RE.replace(folder_name, "").trim().to_string();
    let valid_name = if clean_name.is_empty() {
        folder_name
    } else {
        &clean_name
    };

    if target_enabled {
        return valid_name.to_string();
    }

    format!("DISABLED {valid_name}")
}

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

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, String> {
    let src = Path::new(&path);
    if !src.exists() || !src.is_dir() {
        return Err(format!("Mod folder does not exist: {path}"));
    }

    let parent = src.parent().unwrap_or_else(|| Path::new(""));
    let old_name = src.file_name().unwrap_or_default().to_string_lossy();

    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        return Ok(path);
    }

    let new_path = parent.join(&new_name);

    // Guard: target already exists → rename collision (both X and DISABLED X on disk)
    if new_path.exists() {
        let base = crate::services::scanner::core::normalizer::normalize_display_name(&old_name);
        return Err(format!(
            r#"{{"type":"RenameConflict","attempted_target":"{}","existing_path":"{}","base_name":"{}"}}"#,
            new_path
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
            new_path
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
            base.replace('"', "\\\""),
        ));
    }

    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, &new_path)
            .map_err(|e| format!("Failed to rename mod folder: {e}"))?;
    }

    log::info!("Toggled mod: '{}' -> '{}'", old_name, new_path.display());

    Ok(new_path.to_string_lossy().to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub new_name: String,
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

pub async fn rename_mod_folder_inner(
    state: &WatcherState,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, String> {
    let path = Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Folder does not exist: {folder_path}"));
    }

    if new_name.is_empty() || new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Invalid folder name — contains reserved characters".to_string());
    }

    let parent = path.parent().ok_or("Cannot determine parent directory")?;
    let old_folder_name = path
        .file_name()
        .ok_or("Invalid folder name")?
        .to_string_lossy()
        .to_string();

    let new_folder_name =
        if crate::services::scanner::core::normalizer::is_disabled_folder(&old_folder_name) {
            format!("{}{}", crate::DISABLED_PREFIX, new_name)
        } else {
            new_name.clone()
        };

    let new_path = parent.join(&new_folder_name);
    if new_path.exists() {
        return Err(format!(
            "A folder named '{}' already exists",
            new_folder_name
        ));
    }

    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        crate::services::fs_utils::file_utils::rename_cross_drive_fallback(path, &new_path)
            .map_err(|e| format!("Failed to rename folder: {e}"))?;
    }

    update_info_json_name(&new_path, &new_name);

    log::info!("Renamed '{}' -> '{}'", old_folder_name, new_folder_name);

    Ok(RenameResult {
        old_path: folder_path,
        new_path: new_path.to_string_lossy().to_string(),
        new_name,
    })
}

fn update_info_json_name(folder_path: &Path, new_name: &str) {
    use crate::services::mods::info_json;
    if folder_path.join("info.json").exists() {
        let update = info_json::ModInfoUpdate {
            actual_name: Some(new_name.to_string()),
            ..Default::default()
        };
        let _ = info_json::update_info_json(folder_path, &update);
    }
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
