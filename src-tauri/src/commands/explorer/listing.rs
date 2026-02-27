use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::DISABLED_PREFIX;

use super::helpers::analyze_mod_metadata;
use super::types::ModFolder;

/// Read the filesystem, build `ModFolder` entries, then optionally enrich with DB IDs.
/// Missing mods are automatically inserted into the database.
pub(crate) async fn scan_fs_folders(
    target: &Path,
    _mods_path: &Path,
    sub_path: Option<&str>,
) -> Result<Vec<ModFolder>, String> {
    let entries = match std::fs::read_dir(target) {
        Ok(e) => e,
        Err(e) => {
            log::debug!("Could not read directory (may not exist yet): {}", e);
            return Ok(Vec::new());
        }
    };

    let mut folders: Vec<ModFolder> = entries
        .flatten()
        .filter_map(|entry| build_mod_folder_from_fs_entry(entry, sub_path))
        .collect();

    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(folders)
}

/// Builds a `ModFolder` from a filesystem `DirEntry`. Returns `None` if the entry
/// should be skipped (non-directory, hidden, or no file name).
pub(crate) fn build_mod_folder_from_fs_entry(
    entry: std::fs::DirEntry,
    sub_path: Option<&str>,
) -> Option<ModFolder> {
    let path = entry.path();
    if !path.is_dir() {
        return None;
    }

    let folder_name = path.file_name()?.to_string_lossy().to_string();
    if folder_name.starts_with('.') {
        return None;
    }

    let (is_enabled, display_name) =
        if let Some(stripped) = folder_name.strip_prefix(DISABLED_PREFIX) {
            (false, stripped.to_string())
        } else {
            (true, folder_name.clone())
        };

    // Call metadata once and reuse for both modified_at and size_bytes.
    let entry_meta = entry.metadata().ok();
    let modified_at = entry_meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size_bytes = entry_meta.map(|m| m.len()).unwrap_or(0);

    let info = analyze_mod_metadata(&path, sub_path);
    let (node_type, classification_reasons) = super::classifier::classify_folder(&path);

    Some(ModFolder {
        node_type: node_type.as_str().to_string(),
        classification_reasons,
        id: None,
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
    })
}

pub(crate) async fn list_mod_folders_inner(
    mods_path: String,
    sub_path: Option<String>,
) -> Result<Vec<ModFolder>, String> {
    let base = Path::new(&mods_path);

    if !base.exists() {
        return Err(format!("Mods path does not exist: {mods_path}"));
    }
    if !base.is_dir() {
        return Err(format!("Mods path is not a directory: {mods_path}"));
    }

    log::debug!("Listing mods at base: {}", base.display());

    // Resolve target directory (base + optional sub_path).
    let target = match &sub_path {
        Some(sp) if !sp.is_empty() => base.join(sp),
        _ => base.to_path_buf(),
    };

    // Case-insensitive fallback: even on NTFS with case-sensitivity "disabled",
    // some systems still treat paths case-sensitively. Zero cost when target exists.
    let target = if target.exists() {
        target
    } else if let (Some(parent), Some(name)) = (target.parent(), target.file_name()) {
        let needle = name.to_string_lossy().to_lowercase();
        std::fs::read_dir(parent)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .find(|e| e.file_name().to_string_lossy().to_lowercase() == needle)
            })
            .map(|e| e.path())
            .unwrap_or(target)
    } else {
        target
    };

    log::info!("Scanning filesystem for mods at {}", target.display());

    let folders = scan_fs_folders(&target, base, sub_path.as_deref()).await?;

    log::info!(
        "Listed {} mod folders from {} (sub: {:?})",
        folders.len(),
        mods_path,
        sub_path
    );

    Ok(folders)
}
