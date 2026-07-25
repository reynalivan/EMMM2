use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::common::normalizer::{is_disabled_folder, normalize_display_name};

use crate::services::explorer::helpers::analyze_mod_metadata;
use crate::services::explorer::types::ModFolder;

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
        (false, normalize_display_name(&folder_name))
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
        crate::common::classifier::classify_folder(path);

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
