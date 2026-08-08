use std::path::Path;

use crate::common::normalizer::{is_disabled_folder, normalize_display_name};
use crate::common::path_key::canonical_name_key;
use crate::domain::errors::AppError;

use crate::services::explorer::types::ModFolder;

use super::builder::build_mod_folder_from_fs_entry;

/// Scans each segment of `sub_path` for a `DISABLED ` prefix.
///
/// Returns the **display name** (prefix stripped) of the nearest disabled
/// ancestor segment, or `None` if the path is fully enabled.
///
/// - O(depth) — no filesystem I/O, no DB queries.
/// - Multi-level aware: returns the first (outermost) disabled segment.
///
pub fn find_disabled_ancestor(mods_path: &str, sub_path: &str) -> Option<(String, String)> {
    let base = Path::new(mods_path);
    let mut current = base.to_path_buf();

    for segment in sub_path.split(['/', '\\']) {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }
        current = current.join(trimmed);
        if is_disabled_folder(trimmed) {
            return Some((
                normalize_display_name(trimmed).into_owned(),
                current.to_string_lossy().to_string(),
            ));
        }
    }
    None
}

/// Reads `target` and builds a `ModFolder` per visible child directory, sorted
/// by display name. Filesystem only — no DB access, no writes.
///
/// A directory that cannot be read yields an empty listing rather than an
/// error: the caller may be pointed at a folder the user just removed.
pub fn scan_fs_folders(target: &Path, sub_path: Option<&str>) -> Result<Vec<ModFolder>, AppError> {
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

    // Cached keys: `sort_by_key` would re-run the normalizer regex O(n log n) times.
    folders.sort_by_cached_key(|folder| canonical_name_key(&folder.name));

    Ok(folders)
}
