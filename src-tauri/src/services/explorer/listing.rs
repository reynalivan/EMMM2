use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};

use crate::services::explorer::helpers::analyze_mod_metadata;
use crate::services::explorer::types::ModFolder;

/// Read the filesystem, build `ModFolder` entries, then optionally enrich with DB IDs.
/// Missing mods are automatically inserted into the database.
pub async fn scan_fs_folders(
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
pub fn build_mod_folder_from_fs_entry(
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

    let (is_enabled, display_name) = if is_disabled_folder(&folder_name) {
        (false, normalize_display_name(&folder_name))
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
    let (node_type, classification_reasons) =
        crate::services::explorer::classifier::classify_folder(&path);

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
        conflict_group_id: None,
        conflict_state: None,
    })
}

pub async fn list_mod_folders_inner(
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::services::explorer::types::FolderGridResponse, String> {
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

    let mut folders = scan_fs_folders(&target, base, sub_path.as_deref()).await?;

    log::info!(
        "Listed {} mod folders from {} (sub: {:?})",
        folders.len(),
        mods_path,
        sub_path
    );

    // ── Conflict grouping pass (O(n)) ────────────────────────────────────────
    // Group folders by normalized base name (stripped of DISABLED prefix, lowercased).
    // If a group has >1 member → conflict (e.g. both "X" and "DISABLED X" exist).
    use crate::services::explorer::types::{ConflictGroup, ConflictMember};
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, f) in folders.iter().enumerate() {
        let base_key = normalize_display_name(&f.folder_name).to_lowercase();
        groups.entry(base_key).or_default().push(i);
    }

    let mut conflicts: Vec<ConflictGroup> = Vec::new();
    for (base_key, indices) in &groups {
        if indices.len() < 2 {
            continue;
        }
        // Compute stable group_id from parent path + base_key
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        target.to_string_lossy().hash(&mut hasher);
        base_key.hash(&mut hasher);
        let group_id = format!("cg_{:016x}", hasher.finish());

        let base_name = normalize_display_name(&folders[indices[0]].folder_name);

        let members: Vec<ConflictMember> = indices
            .iter()
            .map(|&i| {
                let f = &folders[i];
                ConflictMember {
                    path: f.path.clone(),
                    folder_name: f.folder_name.clone(),
                    is_enabled: f.is_enabled,
                    modified_at: f.modified_at,
                    size_bytes: f.size_bytes,
                }
            })
            .collect();

        // Annotate each folder in the conflict group
        for &i in indices {
            folders[i].conflict_group_id = Some(group_id.clone());
            folders[i].conflict_state = Some("EnabledDisabledBothPresent".to_string());
        }

        conflicts.push(ConflictGroup {
            group_id,
            base_name,
            members,
        });
    }

    // ── Self-sibling conflict check ──────────────────────────────────────────
    // When navigated into a sub_path (e.g. "stelle_simple_black_v1_00"),
    // check the parent directory for a sibling with the opposite DISABLED prefix.
    // This catches the common case where both "X" and "DISABLED X" exist as
    // siblings at the Mods root but the user is viewing inside one of them.
    if let Some(sp) = &sub_path {
        if !sp.is_empty() {
            let self_name = target
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let self_base = normalize_display_name(&self_name);
            let self_disabled = is_disabled_folder(&self_name);

            // Build the expected sibling name (toggle the prefix)
            let sibling_name = if self_disabled {
                // Self is disabled → look for the enabled version (no prefix)
                self_base.clone()
            } else {
                // Self is enabled → look for the disabled version
                format!("{}{}", crate::DISABLED_PREFIX, self_name)
            };

            let parent_dir = target.parent().unwrap_or(&target);
            let sibling_path = parent_dir.join(&sibling_name);

            if sibling_path.exists() && sibling_path.is_dir() {
                // Found a sibling conflict! Build a ConflictGroup for it.
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                parent_dir.to_string_lossy().hash(&mut hasher);
                self_base.to_lowercase().hash(&mut hasher);
                let group_id = format!("cg_{:016x}", hasher.finish());

                let self_meta = std::fs::metadata(&target);
                let sibling_meta = std::fs::metadata(&sibling_path);

                let self_member = ConflictMember {
                    path: target.to_string_lossy().to_string(),
                    folder_name: self_name.clone(),
                    is_enabled: !self_disabled,
                    modified_at: self_meta
                        .as_ref()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    size_bytes: self_meta.as_ref().ok().map(|m| m.len()).unwrap_or(0),
                };

                let sibling_member = ConflictMember {
                    path: sibling_path.to_string_lossy().to_string(),
                    folder_name: sibling_name.clone(),
                    is_enabled: self_disabled, // opposite of self
                    modified_at: sibling_meta
                        .as_ref()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    size_bytes: sibling_meta.as_ref().ok().map(|m| m.len()).unwrap_or(0),
                };

                conflicts.push(ConflictGroup {
                    group_id,
                    base_name: self_base,
                    members: vec![self_member, sibling_member],
                });
            }
        }
    }

    let (self_node_type, self_classification_reasons) =
        crate::services::explorer::classifier::classify_folder(&target);
    let self_is_mod = self_node_type
        == crate::services::explorer::classifier::NodeType::FlatModRoot
        || self_node_type == crate::services::explorer::classifier::NodeType::ModPackRoot;

    // Determine self_is_enabled based on the final path directory component prefix
    let self_is_enabled = if let Some(sp) = &sub_path {
        let name = Path::new(sp)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        !is_disabled_folder(name)
    } else {
        true
    };

    Ok(crate::services::explorer::types::FolderGridResponse {
        self_node_type: Some(self_node_type.as_str().to_string()),
        self_is_mod,
        self_is_enabled,
        self_classification_reasons,
        children: folders,
        conflicts,
    })
}

#[cfg(test)]
#[path = "tests/listing_tests.rs"]
mod tests;
