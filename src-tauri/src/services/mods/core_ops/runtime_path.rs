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
        if crate::common::path_key::names_equal_by_key(&entry_name, segment) {
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
