//! Resolving an on-disk folder that may exist under either enabled or
//! disabled naming variant.

use super::naming::standardize_prefix;
use std::path::{Path, PathBuf};

fn path_components_as_strings(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect()
}

fn find_runtime_variant_child(
    parent: &Path,
    segment: &str,
    desired_enabled: bool,
) -> Option<PathBuf> {
    let preferred_name = standardize_prefix(segment, desired_enabled);
    let preferred = parent.join(&preferred_name);
    if preferred.is_dir() {
        return Some(preferred);
    }

    let entries = std::fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let entry_name = entry.file_name().to_string_lossy().to_string();
        if crate::shared::path_key::names_equal_by_key(&entry_name, segment) {
            return Some(entry_path);
        }
    }

    None
}

pub(crate) fn resolve_existing_runtime_variant(
    mods_root: &Path,
    absolute_target: &Path,
    desired_enabled: bool,
) -> Option<PathBuf> {
    if absolute_target.is_dir() {
        return Some(absolute_target.to_path_buf());
    }

    let relative = absolute_target.strip_prefix(mods_root).ok()?;
    let components = path_components_as_strings(relative);
    if components.is_empty() {
        return None;
    }

    let mut current = mods_root.to_path_buf();
    for (index, segment) in components.iter().enumerate() {
        let direct = current.join(segment);
        if direct.is_dir() {
            current = direct;
            continue;
        }

        let segment_desired_enabled = if index + 1 == components.len() {
            desired_enabled
        } else {
            true
        };
        current = find_runtime_variant_child(&current, segment, segment_desired_enabled)?;
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::resolve_existing_runtime_variant;
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;

    #[test]
    fn stale_disabled_spelling_follows_one_physical_folder_through_rapid_toggles() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let disabled = root.join("DISABLED Blue");
        let enabled = root.join("Blue");
        std::fs::create_dir(&disabled).expect("source");

        for desired_enabled in [true, false, true] {
            let resolved = resolve_existing_runtime_variant(root, &disabled, desired_enabled)
                .expect("stale path resolves on disk");
            let plan = plan_toggle_rename(&resolved, desired_enabled)
                .expect("a folder must not collide with itself");
            if let Some(plan) = plan {
                plan.apply("mod folder").expect("rename");
            }
            let physical_count = std::fs::read_dir(root).expect("root").count();
            assert_eq!(physical_count, 1);
            assert_eq!(enabled.is_dir(), desired_enabled);
            assert_eq!(disabled.is_dir(), !desired_enabled);
        }
    }
}
