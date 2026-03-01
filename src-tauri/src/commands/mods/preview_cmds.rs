use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::ini::document::{self as ini_document, IniDocument};
use crate::services::ini::write as ini_write;
use crate::services::mods::preview_image;
use crate::services::scanner::core::thumbnail;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IniFileEntry {
    pub filename: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IniLineUpdate {
    pub line_idx: usize,
    pub content: String,
}

fn validate_mod_root(folder_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid mod folder path: {folder_path}"));
    }
    path.canonicalize()
        .map_err(|e| format!("Failed to canonicalize mod folder path: {e}"))
}

fn validate_ini_filename(file_name: &str) -> Result<(), String> {
    if file_name.trim().is_empty() {
        return Err("INI filename cannot be empty".to_string());
    }

    let name_path = Path::new(file_name);
    if name_path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::CurDir | Component::RootDir
        )
    }) {
        return Err("Invalid INI filename path".to_string());
    }

    if name_path.components().count() != 1 {
        return Err("INI filename must not include directories".to_string());
    }

    let lower = file_name.to_ascii_lowercase();
    if lower == "desktop.ini" {
        return Err("desktop.ini is not a valid editable INI".to_string());
    }
    if !lower.ends_with(".ini") {
        return Err("Only .ini files are supported".to_string());
    }

    Ok(())
}

fn resolve_ini_path(mod_root: &Path, file_name: &str) -> Result<PathBuf, String> {
    validate_ini_filename(file_name)?;

    let canonical_root = mod_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize mod folder path: {e}"))?;

    let target = canonical_root.join(file_name);
    let canonical_target = target
        .canonicalize()
        .map_err(|e| format!("Failed to resolve INI file path: {e}"))?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err("INI file path escapes mod folder".to_string());
    }

    if !canonical_target.is_file() {
        return Err(format!(
            "INI file not found: {}",
            canonical_target.display()
        ));
    }

    Ok(canonical_target)
}

fn resolve_image_path(mod_root: &Path, image_path: &str) -> Result<PathBuf, String> {
    if image_path.trim().is_empty() {
        return Err("Image path cannot be empty".to_string());
    }

    let canonical_root = mod_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize mod folder path: {e}"))?;

    let raw = Path::new(image_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        canonical_root.join(raw)
    };

    let canonical_target = candidate
        .canonicalize()
        .map_err(|e| format!("Failed to resolve image path: {e}"))?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err("Image path escapes mod folder".to_string());
    }

    Ok(canonical_target)
}

pub fn list_mod_ini_files_inner(mod_root: &Path) -> Result<Vec<IniFileEntry>, String> {
    let files = ini_document::list_ini_files(mod_root)?;
    Ok(files
        .into_iter()
        .map(|path| IniFileEntry {
            filename: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: path.to_string_lossy().to_string(),
        })
        .collect())
}

pub fn read_mod_ini_inner(mod_root: &Path, file_name: &str) -> Result<IniDocument, String> {
    let ini_path = resolve_ini_path(mod_root, file_name)?;
    ini_document::read_ini_document(&ini_path)
}

pub fn write_mod_ini_inner(
    mod_root: &Path,
    file_name: &str,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), String> {
    let ini_path = resolve_ini_path(mod_root, file_name)?;
    let document = ini_document::read_ini_document(&ini_path)?;
    let updates: Vec<(usize, String)> = line_updates
        .into_iter()
        .map(|u| (u.line_idx, u.content))
        .collect();

    ini_write::save_ini_with_updates(&document, &updates)
}

pub async fn write_mod_ini_locked_inner(
    op_lock: &OperationLock,
    mod_root: &Path,
    file_name: &str,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), String> {
    let _lock = op_lock.acquire().await?;
    write_mod_ini_inner(mod_root, file_name, line_updates)
}

pub fn list_mod_preview_images_inner(mod_root: &Path) -> Result<Vec<String>, String> {
    if !mod_root.exists() || !mod_root.is_dir() {
        return Err(format!("Invalid mod folder: {}", mod_root.display()));
    }

    Ok(thumbnail::list_preview_images(mod_root)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

pub fn save_mod_preview_image_inner(
    mod_root: &Path,
    object_name: &str,
    image_data: &[u8],
) -> Result<String, String> {
    let saved = preview_image::save_preview_image(mod_root, object_name, image_data)?;
    Ok(saved.to_string_lossy().to_string())
}

pub fn remove_mod_preview_image_inner(mod_root: &Path, image_path: &str) -> Result<(), String> {
    let target = resolve_image_path(mod_root, image_path)?;
    preview_image::remove_preview_image(mod_root, &target)
}

pub fn clear_mod_preview_images_inner(mod_root: &Path) -> Result<Vec<String>, String> {
    preview_image::clear_preview_images(mod_root)
}

#[tauri::command]
pub async fn list_mod_ini_files(folder_path: String) -> Result<Vec<IniFileEntry>, String> {
    let mod_root = validate_mod_root(&folder_path)?;
    list_mod_ini_files_inner(&mod_root)
}

#[tauri::command]
pub async fn read_mod_ini(folder_path: String, file_name: String) -> Result<IniDocument, String> {
    let mod_root = validate_mod_root(&folder_path)?;
    read_mod_ini_inner(&mod_root, &file_name)
}

#[tauri::command]
pub async fn write_mod_ini(
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    file_name: String,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), String> {
    let mod_root = validate_mod_root(&folder_path)?;
    write_mod_ini_locked_inner(&op_lock, &mod_root, &file_name, line_updates).await
}

#[tauri::command]
pub async fn list_mod_preview_images(folder_path: String) -> Result<Vec<String>, String> {
    let mod_root = validate_mod_root(&folder_path)?;
    list_mod_preview_images_inner(&mod_root)
}

#[tauri::command]
pub async fn save_mod_preview_image(
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    object_name: String,
    image_data: Vec<u8>,
) -> Result<String, String> {
    const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

    if image_data.len() > MAX_IMAGE_BYTES {
        return Err("Image too large. Max 10MB.".to_string());
    }

    let mod_root = validate_mod_root(&folder_path)?;
    let _lock = op_lock.acquire().await?;
    save_mod_preview_image_inner(&mod_root, &object_name, &image_data)
}

#[tauri::command]
pub async fn remove_mod_preview_image(
    op_lock: State<'_, OperationLock>,
    folder_path: String,
    image_path: String,
) -> Result<(), String> {
    let mod_root = validate_mod_root(&folder_path)?;
    let _lock = op_lock.acquire().await?;
    remove_mod_preview_image_inner(&mod_root, &image_path)
}

#[tauri::command]
pub async fn clear_mod_preview_images(
    op_lock: State<'_, OperationLock>,
    folder_path: String,
) -> Result<Vec<String>, String> {
    let mod_root = validate_mod_root(&folder_path)?;
    let _lock = op_lock.acquire().await?;
    clear_mod_preview_images_inner(&mod_root)
}

#[cfg(test)]
#[path = "tests/preview_cmds_tests.rs"]
mod tests;
