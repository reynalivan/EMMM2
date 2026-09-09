//! Folder-name normalization and rename-conflict detection.

use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};

/// Validate the user-editable base name used by every folder rename flow.
/// Enabled/disabled state is owned by the source path and must never be
/// smuggled into this value.
pub(crate) fn validate_folder_base_name(name: &str) -> Result<(), AppError> {
    validate_folder_name_component(name)?;
    if crate::modules::workspace::domain::normalizer::is_disabled_folder(name) {
        return Err(AppError::Io(
            "Folder base name must not include the DISABLED prefix".to_string(),
        ));
    }

    Ok(())
}

/// Validate one filesystem name component. This is shared by rename and
/// archive-placement flows; callers separately own enabled/disabled status.
pub(crate) fn validate_folder_name_component(name: &str) -> Result<(), AppError> {
    if name.is_empty() || name.trim() != name {
        return Err(AppError::Io(
            "Folder name cannot be empty or padded".to_string(),
        ));
    }
    if name
        .chars()
        .any(|character| character.is_control() || r#"/\:*?"<>|"#.contains(character))
        || name.ends_with(['.', ' '])
    {
        return Err(AppError::Io(
            "Invalid folder name — contains characters reserved by Windows".to_string(),
        ));
    }

    let device = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || device
            .strip_prefix("COM")
            .or_else(|| device.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            });
    if reserved {
        return Err(AppError::Io(
            "Folder name is reserved by Windows".to_string(),
        ));
    }

    Ok(())
}

pub fn standardize_prefix(folder_name: &str, target_enabled: bool) -> String {
    let clean_name =
        crate::modules::workspace::domain::normalizer::normalize_display_name(folder_name);
    let valid_name = if clean_name.is_empty() {
        folder_name.trim()
    } else {
        &clean_name
    };

    if target_enabled {
        return valid_name.to_string();
    }

    format!("{}{valid_name}", crate::DISABLED_PREFIX)
}

pub(crate) fn find_existing_sibling_case_insensitive(
    parent: &Path,
    target_name: &str,
    source_path: &Path,
) -> Option<PathBuf> {
    find_sibling_identity_collision(parent, target_name, Some(source_path))
}

pub(crate) fn find_sibling_identity_collision(
    parent: &Path,
    target_name: &str,
    source_path: Option<&Path>,
) -> Option<PathBuf> {
    let target_identity =
        crate::modules::workspace::domain::normalizer::normalize_display_name(target_name);
    let entries = std::fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if source_path.is_some_and(|source_path| entry_path == source_path) {
            continue;
        }

        let entry_name = entry.file_name();
        let entry_name = entry_name.to_string_lossy();
        let exact_collision = entry_name.eq_ignore_ascii_case(target_name);
        let identity_collision = entry_path.is_dir()
            && crate::modules::workspace::domain::normalizer::normalize_display_name(&entry_name)
                .eq_ignore_ascii_case(&target_identity);
        if exact_collision || identity_collision {
            return Some(entry_path);
        }
    }

    None
}

pub(crate) fn rename_conflict_error(
    attempted_path: &Path,
    existing_path: &Path,
    base_name: &str,
) -> AppError {
    AppError::Io(
        serde_json::json!({
            "type": "RenameConflict",
            "message": "Target already exists",
            "attempted_target": attempted_path.to_string_lossy(),
            "existing_path": existing_path.to_string_lossy(),
            "base_name": base_name,
        })
        .to_string(),
    )
}
