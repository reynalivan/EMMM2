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
    let target_identity =
        crate::modules::workspace::domain::normalizer::normalize_display_name(target_name);
    std::fs::read_dir(parent).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        if path == source_path {
            return None;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let collision = name.eq_ignore_ascii_case(target_name)
            || entry.file_type().ok().is_some_and(|kind| kind.is_dir())
                && crate::modules::workspace::domain::normalizer::normalize_display_name(&name)
                    .eq_ignore_ascii_case(&target_identity);
        if !collision {
            return None;
        }
        (!same_filesystem_entry(source_path, &path)).then_some(path)
    })
}

pub(crate) fn find_existing_destination_case_insensitive(
    parent: &Path,
    target_name: &str,
    source_path: &Path,
) -> Option<PathBuf> {
    std::fs::read_dir(parent).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        if path == source_path
            || !entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(target_name)
        {
            return None;
        }
        (!same_filesystem_entry(source_path, &path)).then_some(path)
    })
}

fn same_filesystem_entry(source: &Path, other: &Path) -> bool {
    crate::platform::fs::file_utils::filesystem_identity(source).is_some_and(|identity| {
        crate::platform::fs::file_utils::filesystem_identity(other).as_deref()
            == Some(identity.as_str())
    })
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
    destination_names: std::collections::HashMap<String, Vec<usize>>,
}

impl SiblingNameIndex {
    pub fn read(parent: &Path) -> Option<Self> {
        let entries = std::fs::read_dir(parent)
            .ok()?
            .flatten()
            .map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let normalized_directory_name = entry
                    .file_type()
                    .ok()
                    .filter(|kind| kind.is_dir())
                    .map(|_| {
                        crate::modules::workspace::domain::normalizer::normalize_display_name(&name)
                            .to_string()
                    });
                SiblingNameEntry {
                    path,
                    name,
                    normalized_directory_name,
                }
            })
            .collect::<Vec<_>>();
        Some(Self::from_entries(entries))
    }

    fn from_entries(entries: Vec<SiblingNameEntry>) -> Self {
        let mut destination_names = std::collections::HashMap::<String, Vec<usize>>::new();
        for (index, entry) in entries.iter().enumerate() {
            destination_names
                .entry(entry.name.to_ascii_lowercase())
                .or_default()
                .push(index);
        }
        Self {
            entries,
            destination_names,
        }
    }

    pub fn find_destination_collision(
        &self,
        target_name: &str,
        source_path: &Path,
    ) -> Option<PathBuf> {
        self.destination_names
            .get(&target_name.to_ascii_lowercase())?
            .iter()
            .find_map(|index| {
                let entry = &self.entries[*index];
                (entry.path != source_path && !same_filesystem_entry(source_path, &entry.path))
                    .then(|| entry.path.clone())
            })
    }

    fn find_collision_excluding(
        &self,
        target_name: &str,
        source_path: Option<&Path>,
    ) -> Option<PathBuf> {
        let target_identity =
            crate::modules::workspace::domain::normalizer::normalize_display_name(target_name);
        self.entries.iter().find_map(|entry| {
            let exact_collision = entry.name.eq_ignore_ascii_case(target_name);
            let identity_collision = entry
                .normalized_directory_name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(&target_identity));
            if source_path.is_some_and(|source_path| {
                entry.path == source_path
                    || (exact_collision || identity_collision)
                        && same_filesystem_entry(source_path, &entry.path)
            }) {
                return None;
            }
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

#[cfg(test)]
mod tests {
    #[test]
    fn bulk_destination_index_has_one_matching_bucket_among_ten_thousand_siblings() {
        let entries = (0..10_000)
            .map(|index| super::SiblingNameEntry {
                path: std::path::PathBuf::from(format!("Mods/Mod-{index:05}")),
                name: format!("Mod-{index:05}"),
                normalized_directory_name: None,
            })
            .collect();
        let index = super::SiblingNameIndex::from_entries(entries);
        assert_eq!(index.destination_names.get("mod-09999").unwrap().len(), 1);
        assert_eq!(
            index.find_destination_collision("MOD-09999", std::path::Path::new("Mods/Other")),
            Some(std::path::PathBuf::from("Mods/Mod-09999"))
        );
        assert!(index
            .find_destination_collision("Missing", std::path::Path::new("Mods/Other"))
            .is_none());
    }

    #[test]
    fn destination_index_retains_all_casefold_candidates_and_excludes_source() {
        let index = super::SiblingNameIndex::from_entries(vec![
            super::SiblingNameEntry {
                path: std::path::PathBuf::from("Mods/Skin"),
                name: "Skin".to_string(),
                normalized_directory_name: None,
            },
            super::SiblingNameEntry {
                path: std::path::PathBuf::from("Mods/SKIN"),
                name: "SKIN".to_string(),
                normalized_directory_name: None,
            },
        ]);
        assert_eq!(index.destination_names.get("skin").unwrap().len(), 2);
        assert_eq!(
            index.find_destination_collision("Skin", std::path::Path::new("Mods/Skin")),
            Some(std::path::PathBuf::from("Mods/SKIN"))
        );
    }

    #[test]
    fn toggle_rejects_destination_created_after_planning_without_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("Skin");
        std::fs::create_dir(&source).unwrap();
        let plan = super::super::toggle::plan_toggle_rename(&source, false)
            .unwrap()
            .unwrap();
        std::fs::create_dir(plan.new_path()).unwrap();
        std::fs::write(plan.new_path().join("foreign"), "preserve").unwrap();
        let identity = crate::platform::fs::file_utils::filesystem_identity(&source);
        let error = plan.apply("mod folder").unwrap_err();
        assert!(error.to_string().contains("RenameConflict"));
        assert_eq!(
            crate::platform::fs::file_utils::filesystem_identity(&source),
            identity
        );
        assert_eq!(
            std::fs::read_to_string(plan.new_path().join("foreign")).unwrap(),
            "preserve"
        );
    }
    #[test]
    fn toggle_accepts_free_destination_beside_distinct_prefix_variant() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("DISABLED A");
        let sibling = root.path().join("DISABLED DISABLED A");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&sibling).unwrap();
        let sibling_identity = crate::platform::fs::file_utils::filesystem_identity(&sibling);
        let index = super::SiblingNameIndex::read(root.path()).unwrap();
        let plan = super::super::toggle::plan_toggle_rename_with_sibling_index(
            &source,
            true,
            Some(&index),
        )
        .unwrap()
        .unwrap();
        plan.apply("mod folder").unwrap();
        assert!(root.path().join("A").is_dir());
        assert!(!source.exists());
        assert_eq!(
            crate::platform::fs::file_utils::filesystem_identity(&sibling),
            sibling_identity
        );
    }

    #[test]
    fn toggle_rejects_actual_destination_file_and_folder_without_overwrite() {
        for directory in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("DISABLED A");
            let target = root.path().join("A");
            std::fs::create_dir(&source).unwrap();
            if directory {
                std::fs::create_dir(&target).unwrap();
            } else {
                std::fs::write(&target, "foreign").unwrap();
            }
            let source_id = crate::platform::fs::file_utils::filesystem_identity(&source);
            let target_id = crate::platform::fs::file_utils::filesystem_identity(&target);
            let index = super::SiblingNameIndex::read(root.path()).unwrap();
            assert!(super::super::toggle::plan_toggle_rename_with_sibling_index(
                &source,
                true,
                Some(&index)
            )
            .is_err());
            assert!(super::super::toggle::plan_toggle_rename(&source, true).is_err());
            assert_eq!(
                crate::platform::fs::file_utils::filesystem_identity(&source),
                source_id
            );
            assert_eq!(
                crate::platform::fs::file_utils::filesystem_identity(&target),
                target_id
            );
        }
    }
}
