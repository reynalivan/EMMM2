use crate::services::core::operation_lock::OperationLock;
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
    use sqlx::Row;

    let row = sqlx::query("SELECT id, folder_path FROM mods WHERE object_id = ? LIMIT 1")
        .bind(object_id)
        .fetch_optional(pool)
        .await
        .ok()??;

    let mod_id: String = row.try_get("id").ok()?;
    let folder_path: String = row.try_get("folder_path").ok()?;
    let path = Path::new(&folder_path);

    if path.exists() {
        return Some(folder_path);
    }

    use crate::commands::explorer::try_resolve_alternate;
    if let Some((alt_path, alt_enabled)) = try_resolve_alternate(path) {
        let alt_str = alt_path.to_string_lossy().to_string();
        let new_status = if alt_enabled { "ENABLED" } else { "DISABLED" };

        let _ = sqlx::query("UPDATE mods SET folder_path = ?, status = ? WHERE id = ?")
            .bind(&alt_str)
            .bind(new_status)
            .bind(&mod_id)
            .execute(pool)
            .await;

        log::info!(
            "Self-healed mod {} for reveal: {} → {}",
            mod_id,
            folder_path,
            alt_str
        );
        return Some(alt_str);
    }

    let _ = sqlx::query("DELETE FROM mods WHERE id = ?")
        .bind(&mod_id)
        .execute(pool)
        .await;

    log::warn!(
        "Deleted stale mod {} (folder gone): {}",
        mod_id,
        folder_path
    );
    None
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
) -> Result<String, String> {
    let _lock = op_lock.acquire().await?;
    let new_path = toggle_mod_inner(&state, path.clone(), enable).await?;
    let new_status = if enable { "ENABLED" } else { "DISABLED" };
    let _ = sqlx::query("UPDATE mods SET folder_path = ?, status = ? WHERE folder_path = ?")
        .bind(&new_path)
        .bind(new_status)
        .bind(&path)
        .execute(&*pool)
        .await;
    Ok(new_path)
}

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, String> {
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let parent = path_obj.parent().ok_or("Invalid path parent")?;
    let file_name = path_obj
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();

    let new_name = standardize_prefix(&file_name, enable);
    if new_name == file_name {
        return Ok(path);
    }

    let new_path = parent.join(&new_name);

    if new_path.exists() && file_name.to_lowercase() != new_name.to_lowercase() {
        return Err(format!(
            "Target path already exists: {}",
            new_path.display()
        ));
    }

    let _guard = SuppressionGuard::new(&state.suppressor);
    fs::rename(&path, &new_path).map_err(|e| format!("Failed to rename: {}", e))?;

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
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, String> {
    let _lock = op_lock.acquire().await?;
    rename_mod_folder_inner(&state, folder_path, new_name).await
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

    let new_folder_name = if old_folder_name.starts_with(crate::DISABLED_PREFIX) {
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
        fs::rename(path, &new_path).map_err(|e| format!("Failed to rename folder: {e}"))?;
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
    use crate::services::mod_files::info_json;
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
