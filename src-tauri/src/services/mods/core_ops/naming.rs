//! Folder-name normalization and rename-conflict detection.

use crate::domain::errors::AppError;
use std::path::{Path, PathBuf};

pub fn standardize_prefix(folder_name: &str, target_enabled: bool) -> String {
    let clean_name = crate::common::normalizer::normalize_display_name(folder_name);
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
    let entries = std::fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path == source_path {
            continue;
        }

        let entry_name = entry.file_name();
        if entry_name
            .to_string_lossy()
            .eq_ignore_ascii_case(target_name)
        {
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
