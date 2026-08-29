use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::modules::workspace::domain::normalizer::{is_disabled_folder, normalize_display_name};

use crate::modules::workspace::application::explorer::helpers::analyze_mod_metadata;
use crate::modules::workspace::application::explorer::types::ModFolder;

/// Builds a `ModFolder` from a filesystem `DirEntry`. Returns `None` if the entry
/// should be skipped (non-directory, hidden, or no file name).
fn build_mod_folder_with_path(
    path: &Path,
    sub_path: Option<&str>,
    entry_meta: Option<std::fs::Metadata>,
) -> Option<ModFolder> {
    if !path.is_dir() {
        return None;
    }

    let folder_name = path.file_name()?.to_string_lossy().to_string();
    if folder_name.starts_with('.') {
        return None;
    }

    let (is_enabled, display_name) = if is_disabled_folder(&folder_name) {
        (false, normalize_display_name(&folder_name).into_owned())
    } else {
        (true, folder_name.clone())
    };

    // Call metadata once and reuse for both modified_at and size_bytes.
    let modified_at = entry_meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size_bytes = entry_meta.map(|m| m.len()).unwrap_or(0);

    let info = analyze_mod_metadata(path, sub_path);
    let (node_type, classification_reasons, warnings) =
        crate::modules::workspace::domain::classifier::classify_folder(path);

    Some(ModFolder {
        node_type: node_type.as_str().to_string(),
        classification_reasons,
        id: None,
        owner_object_id: None,
        owner_object_folder_path: None,
        name: display_name,
        folder_name,
        path: path.to_string_lossy().to_string(),
        is_enabled,
        is_directory: true,
        thumbnail_path: None,
        modified_at,
        size_bytes,
        has_info_json: info.has_info_json,
        is_favorite: info.is_favorite,
        is_misplaced: info.is_misplaced,
        is_safe: info.is_safe,
        is_safety_classified: info.has_info_json,
        contains_safe_mods: false,
        contains_unsafe_mods: false,
        metadata: info.metadata,
        category: info.category,
        conflict_group_id: None,
        conflict_state: None,
        warnings,
    })
}

pub fn build_mod_folder_from_path(path: &Path, sub_path: Option<&str>) -> Option<ModFolder> {
    let entry_meta = std::fs::metadata(path).ok();
    build_mod_folder_with_path(path, sub_path, entry_meta)
}

pub fn build_mod_folder_from_fs_entry(
    entry: std::fs::DirEntry,
    sub_path: Option<&str>,
) -> Option<ModFolder> {
    let path = entry.path();
    let entry_meta = entry.metadata().ok();
    build_mod_folder_with_path(&path, sub_path, entry_meta)
}

/// Fast first paint while Disk Reconcile owns the authoritative deep walk.
/// It deliberately avoids classifying folders, reading INI files, and parsing
/// `info.json`; those fields are refreshed by the normal listing afterwards.
pub fn build_mod_folder_shallow_from_fs_entry(entry: std::fs::DirEntry) -> Option<ModFolder> {
    let path = entry.path();
    if !entry.file_type().ok()?.is_dir() {
        return None;
    }

    let folder_name = path.file_name()?.to_string_lossy().to_string();
    if folder_name.starts_with('.') {
        return None;
    }
    let (is_enabled, name) = if is_disabled_folder(&folder_name) {
        (false, normalize_display_name(&folder_name).into_owned())
    } else {
        (true, folder_name.clone())
    };
    Some(ModFolder {
        node_type: crate::modules::workspace::domain::classifier::NodeType::ContainerFolder
            .as_str()
            .to_string(),
        classification_reasons: Vec::new(),
        id: None,
        owner_object_id: None,
        owner_object_folder_path: None,
        name,
        folder_name,
        path: path.to_string_lossy().to_string(),
        is_enabled,
        is_directory: true,
        thumbnail_path: None,
        modified_at: 0,
        size_bytes: 0,
        has_info_json: false,
        is_favorite: false,
        is_misplaced: false,
        // Unknown until the shallow listing overlays the DB's last-known
        // classification. Filtered views fail closed during recovery.
        is_safe: false,
        is_safety_classified: false,
        contains_safe_mods: false,
        contains_unsafe_mods: false,
        metadata: None,
        category: None,
        conflict_group_id: None,
        conflict_state: None,
        warnings: Vec::new(),
    })
}
