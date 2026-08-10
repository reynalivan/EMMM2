//! Pure preview/INI file operations for a mod folder.
//! Moved out of `commands::mods::preview_cmds` so the read-model services can
//! use them without depending on the command layer.

use crate::domain::errors::{AppError, MetadataError};
use crate::services::ini::document::{self as ini_document, IniDocument};
use crate::services::ini::write as ini_write;
use crate::services::mods::preview_image;
use crate::services::scanner::core::thumbnail;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
pub struct IniFileEntry {
    pub filename: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
pub struct IniLineUpdate {
    #[specta(type = f64)]
    pub line_idx: usize,
    pub content: String,
}

const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// Rejects oversized pasted/saved preview images.
pub fn ensure_image_size(image_data: &[u8]) -> Result<(), AppError> {
    if image_data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Metadata(MetadataError::Validation(
            "Image too large. Max 10MB.".to_string(),
        )));
    }
    Ok(())
}

fn validate_ini_filename(file_name: &str) -> Result<(), AppError> {
    if file_name.trim().is_empty() {
        return Err(AppError::Metadata(MetadataError::Validation(
            "INI filename cannot be empty".to_string(),
        )));
    }

    let name_path = Path::new(file_name);
    if name_path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::CurDir | Component::RootDir
        )
    }) {
        return Err(AppError::Metadata(MetadataError::Security(
            "Invalid INI filename path".to_string(),
        )));
    }

    let base_name = name_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if base_name.eq_ignore_ascii_case("desktop.ini") {
        return Err(AppError::Metadata(MetadataError::Validation(
            "desktop.ini is not a valid editable INI".to_string(),
        )));
    }
    if !name_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
    {
        return Err(AppError::Metadata(MetadataError::Validation(
            "Only .ini files are supported".to_string(),
        )));
    }

    Ok(())
}

fn resolve_ini_path(mod_root: &Path, file_name: &str) -> Result<PathBuf, AppError> {
    validate_ini_filename(file_name)?;

    let target = mod_root.join(file_name);
    if !target.is_file() {
        return Err(AppError::Metadata(MetadataError::NotFound(format!(
            "INI file not found: {}",
            target.display()
        ))));
    }

    let canonical_root = mod_root.canonicalize()?;
    let canonical_target = target.canonicalize()?;
    if !canonical_target.starts_with(canonical_root) {
        return Err(AppError::Metadata(MetadataError::Security(
            "INI file path escapes mod folder".to_string(),
        )));
    }

    Ok(canonical_target)
}

pub fn resolve_image_path(mod_root: &Path, image_path: &str) -> Result<PathBuf, AppError> {
    if image_path.trim().is_empty() {
        return Err(AppError::Metadata(MetadataError::Validation(
            "Image path cannot be empty".to_string(),
        )));
    }

    let raw = Path::new(image_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        mod_root.join(raw)
    };

    let canonical_root = mod_root.canonicalize().map_err(|e| {
        AppError::Metadata(MetadataError::NotFound(format!(
            "Failed to resolve mod folder: {e}"
        )))
    })?;

    // Use canonicalize to resolve symlinks and '..' if any (though PathGuard should prevent escaping)
    let canonical_target = candidate.canonicalize().map_err(|e| {
        AppError::Metadata(MetadataError::NotFound(format!(
            "Failed to resolve image path: {e}"
        )))
    })?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err(AppError::Metadata(MetadataError::Security(
            "Image path escapes mod folder".to_string(),
        )));
    }

    Ok(canonical_target)
}

pub fn list_mod_ini_files_inner(mod_root: &Path) -> Result<Vec<IniFileEntry>, AppError> {
    let files = ini_document::list_ini_files(mod_root)?;
    Ok(files
        .into_iter()
        .map(|path| IniFileEntry {
            filename: path
                .strip_prefix(mod_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/"),
            path: path.to_string_lossy().to_string(),
        })
        .collect())
}

pub fn read_mod_ini_inner(mod_root: &Path, file_name: &str) -> Result<IniDocument, AppError> {
    let ini_path = resolve_ini_path(mod_root, file_name)?;
    ini_document::read_ini_document(&ini_path)
}

pub fn write_mod_ini_inner(
    mod_root: &Path,
    file_name: &str,
    expected_source_hash: &str,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), AppError> {
    let ini_path = resolve_ini_path(mod_root, file_name)?;
    let document = ini_document::read_ini_document(&ini_path)?;
    let updates: Vec<(usize, String)> = line_updates
        .into_iter()
        .map(|u| (u.line_idx, u.content))
        .collect();

    ini_write::save_ini_with_updates(&document, expected_source_hash, &updates)
}

pub async fn write_mod_ini_locked_inner(
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    mod_root: &Path,
    file_name: &str,
    expected_source_hash: &str,
    line_updates: Vec<IniLineUpdate>,
) -> Result<(), AppError> {
    write_mod_ini_inner(mod_root, file_name, expected_source_hash, line_updates)
}

pub fn list_mod_preview_images_inner(mod_root: &Path) -> Result<Vec<String>, AppError> {
    if !mod_root.exists() || !mod_root.is_dir() {
        return Err(AppError::Metadata(MetadataError::Validation(format!(
            "Invalid mod folder: {}",
            mod_root.display()
        ))));
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
) -> Result<String, AppError> {
    let saved = preview_image::save_preview_image(mod_root, object_name, image_data)?;
    Ok(saved.to_string_lossy().to_string())
}

pub fn remove_mod_preview_image_inner(mod_root: &Path, image_path: &str) -> Result<(), AppError> {
    let target = resolve_image_path(mod_root, image_path)?;
    preview_image::remove_preview_image(mod_root, &target)
}

pub fn clear_mod_preview_images_inner(mod_root: &Path) -> Result<Vec<String>, AppError> {
    preview_image::clear_preview_images(mod_root)
}

#[cfg(test)]
#[path = "tests/preview_ops_tests.rs"]
mod tests;
