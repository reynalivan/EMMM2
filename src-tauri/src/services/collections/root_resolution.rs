use crate::services::explorer::classifier::{classify_folder, NodeType};
use crate::services::mods::core_ops::standardize_prefix;
use crate::services::path_key::{path_file_name_lossy, resolve_collection_path};
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};
use std::path::{Path, PathBuf};

pub(crate) fn is_foldergrid_level_mod_path(folder_path: &str, mods_path: Option<&str>) -> bool {
    let path = Path::new(folder_path);
    let relative = if let Some(root) = mods_path {
        path.strip_prefix(root).unwrap_or(path)
    } else {
        path
    };
    relative.components().count() > 1
}

pub(crate) fn is_effectively_enabled_folder_path(
    folder_path: &str,
    mods_path: Option<&str>,
) -> bool {
    let Some(resolved_path) = resolve_collection_path(folder_path, mods_path) else {
        return false;
    };

    let relative_path = if let Some(root) = mods_path.map(Path::new) {
        resolved_path
            .strip_prefix(root)
            .unwrap_or(resolved_path.as_path())
            .to_path_buf()
    } else {
        resolved_path
    };

    let mut has_segment = false;
    for component in relative_path.components() {
        let segment = component.as_os_str().to_string_lossy();
        if segment.is_empty() {
            continue;
        }
        has_segment = true;
        if is_disabled_folder(&segment) {
            return false;
        }
    }

    has_segment
}

pub(crate) fn resolve_existing_preview_path(
    folder_path: &str,
    mods_path: Option<&str>,
) -> Option<PathBuf> {
    let mods_root = resolve_mods_root(mods_path)?;
    let resolved = resolve_collection_path(folder_path, mods_path)?;
    if resolved.exists() {
        return Some(normalize_preview_path(&resolved));
    }
    if !resolved.starts_with(&mods_root) {
        return None;
    }

    let relative = resolved.strip_prefix(&mods_root).ok()?;
    let mut current = mods_root.clone();

    for component in relative.components() {
        let display_segment = component.as_os_str().to_string_lossy().to_string();
        let exact = current.join(&display_segment);
        if exact.exists() {
            current = exact;
            continue;
        }

        let enabled_segment = standardize_prefix(&display_segment, true);
        let disabled_segment = standardize_prefix(&display_segment, false);
        let mut matched = None;

        for candidate_segment in [
            display_segment.as_str(),
            enabled_segment.as_str(),
            disabled_segment.as_str(),
        ] {
            let candidate = current.join(candidate_segment);
            if candidate.exists() {
                matched = Some(candidate);
                break;
            }
        }

        current = matched?;
    }

    Some(normalize_preview_path(&current))
}

pub(crate) fn find_root_preview_path(path: &Path, mods_root: &Path) -> Option<(PathBuf, NodeType)> {
    if !path.starts_with(mods_root) {
        return None;
    }

    let mut current = path.to_path_buf();
    let mut fallback_root: Option<(PathBuf, NodeType)> = None;

    loop {
        if current == mods_root {
            return fallback_root;
        }

        let (node_type, _) = classify_folder(&current);
        if node_type == NodeType::VariantContainer {
            return Some((current, node_type));
        }
        if fallback_root.is_none()
            && matches!(node_type, NodeType::ModPackRoot | NodeType::FlatModRoot)
        {
            fallback_root = Some((current.clone(), node_type));
        }

        current = current.parent()?.to_path_buf();
    }
}

pub(crate) fn display_name_for_path(path: &str) -> String {
    path_file_name_lossy(Path::new(path))
        .as_deref()
        .map(normalize_display_name)
        .unwrap_or_else(|| path.to_string())
}

fn resolve_mods_root(mods_path: Option<&str>) -> Option<PathBuf> {
    let root = Path::new(mods_path?);
    if root.is_dir() {
        return Some(root.to_path_buf());
    }
    None
}

fn normalize_preview_path(path: &Path) -> PathBuf {
    path.components().collect()
}
