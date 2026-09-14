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

struct SiblingNameEntry {
    path: PathBuf,
    name: String,
    normalized_directory_name: Option<String>,
}

/// A request-scoped snapshot of one parent directory's sibling names. It is
/// intentionally not cached beyond the bulk planning pass: the rename itself
/// remains the final filesystem authority.
pub struct SiblingNameIndex {
    entries: Vec<SiblingNameEntry>,
}

impl SiblingNameIndex {
    pub fn read(parent: &Path) -> Option<Self> {
        let entries = std::fs::read_dir(parent)
            .ok()?
            .flatten()
            .map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let normalized_directory_name = path.is_dir().then(|| {
                    crate::modules::workspace::domain::normalizer::normalize_display_name(&name)
                        .to_string()
                });
                SiblingNameEntry {
                    path,
                    name,
                    normalized_directory_name,
                }
            })
            .collect();
        Some(Self { entries })
    }

    pub fn find_collision(&self, target_name: &str, source_path: &Path) -> Option<PathBuf> {
        self.find_collision_excluding(target_name, Some(source_path))
    }

    fn find_collision_excluding(
        &self,
        target_name: &str,
        source_path: Option<&Path>,
    ) -> Option<PathBuf> {
        let target_identity =
            crate::modules::workspace::domain::normalizer::normalize_display_name(target_name);
        self.entries.iter().find_map(|entry| {
            if source_path.is_some_and(|source_path| entry.path == source_path) {
                return None;
            }
            let exact_collision = entry.name.eq_ignore_ascii_case(target_name);
            let identity_collision = entry
                .normalized_directory_name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(&target_identity));
            (exact_collision || identity_collision).then(|| entry.path.clone())
        })
    }
}

pub(crate) fn find_sibling_identity_collision(
    parent: &Path,
    target_name: &str,
    source_path: Option<&Path>,
) -> Option<PathBuf> {
    let index = SiblingNameIndex::read(parent)?;
    index.find_collision_excluding(target_name, source_path)
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
