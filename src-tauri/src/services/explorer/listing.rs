use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::repo::object_repo::{get_runtime_descriptors, ObjectRuntimeDescriptor};
use crate::services::path_key::{canonical_name_key, names_equal_by_key, path_file_name_lossy};
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};

use crate::services::explorer::helpers::analyze_mod_metadata;
use crate::services::explorer::types::ModFolder;

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
                normalize_display_name(trimmed),
                current.to_string_lossy().to_string(),
            ));
        }
    }
    None
}

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

    folders.sort_by_key(|folder| canonical_name_key(&folder.name));

    Ok(folders)
}

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
        crate::services::explorer::classifier::classify_folder(path);

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

fn resolve_owner_descriptor<'a>(
    owners: &'a [ObjectRuntimeDescriptor],
    folder_path: &str,
    mods_path: &str,
) -> Option<&'a ObjectRuntimeDescriptor> {
    let mut best_match: Option<&ObjectRuntimeDescriptor> = None;
    let mut best_key_length = 0usize;

    for owner in owners {
        if !crate::services::path_key::path_starts_with_key(
            folder_path,
            &owner.folder_path,
            Some(mods_path),
        ) {
            continue;
        }

        let key_length = owner.folder_path_key.len();
        if key_length <= best_key_length {
            continue;
        }

        best_match = Some(owner);
        best_key_length = key_length;
    }

    best_match
}

fn enrich_owner_metadata(
    response: &mut crate::services::explorer::types::FolderGridResponse,
    owners: &[ObjectRuntimeDescriptor],
    mods_path: &str,
    sub_path: Option<&str>,
) {
    for folder in &mut response.children {
        let Some(owner) = resolve_owner_descriptor(owners, &folder.path, mods_path) else {
            continue;
        };
        folder.owner_object_id = Some(owner.id.clone());
        folder.owner_object_folder_path = Some(owner.folder_path.clone());
    }

    let Some(relative_sub_path) = sub_path else {
        return;
    };
    if relative_sub_path.is_empty() {
        return;
    }

    let self_path = Path::new(mods_path).join(relative_sub_path);
    let self_path_str = self_path.to_string_lossy().to_string();
    let Some(owner) = resolve_owner_descriptor(owners, &self_path_str, mods_path) else {
        return;
    };
    response.self_owner_object_id = Some(owner.id.clone());
    response.self_owner_object_folder_path = Some(owner.folder_path.clone());
}

pub async fn list_mod_folders_for_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::services::explorer::types::FolderGridResponse, String> {
    let owners = get_runtime_descriptors(pool, game_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut response = list_mod_folders_inner(mods_path.clone(), sub_path.clone()).await?;
    enrich_owner_metadata(&mut response, &owners, &mods_path, sub_path.as_deref());
    Ok(response)
}

pub async fn list_mod_folders_inner(
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::services::explorer::types::FolderGridResponse, String> {
    let mut base = Path::new(&mods_path).to_path_buf();
    let mut is_root_disabled = false;

    if !base.exists() {
        // Check if the root directory itself is disabled (prefixed with "DISABLED ")
        if let (Some(parent), Some(name)) = (base.parent(), base.file_name()) {
            let disabled_name = format!("{}{}", crate::DISABLED_PREFIX, name.to_string_lossy());
            let disabled_base = parent.join(disabled_name);
            if disabled_base.exists() {
                base = disabled_base;
                is_root_disabled = true;
            }
        }
    }

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
        let needle = name.to_string_lossy().to_string();
        // Fallback 1: Case-insensitive match on exact name
        let case_insensitive_match = std::fs::read_dir(parent)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .find(|e| names_equal_by_key(&e.file_name().to_string_lossy(), &needle))
            })
            .map(|e| e.path());

        if let Some(path) = case_insensitive_match {
            path
        } else {
            // Fallback 2: Check for DISABLED prefix variant
            let disabled_needle = format!("DISABLED {}", needle);
            std::fs::read_dir(parent)
                .ok()
                .and_then(|entries| {
                    entries.flatten().find(|e| {
                        names_equal_by_key(&e.file_name().to_string_lossy(), &disabled_needle)
                    })
                })
                .map(|e| e.path())
                .unwrap_or(target)
        }
    } else {
        target
    };

    // ── Traversal guard ─────────────────────────────────────────────────────────
    // Ensure the resolved target stays inside the declared mods root.
    // A crafted sub_path like "../../etc" could otherwise escape the boundary.
    {
        let canonical_base = base
            .canonicalize()
            .map_err(|e| format!("Cannot canonicalize mods_path: {e}"))?;
        // target may not exist yet if renamed; fall back to the raw path.
        let canonical_target = target.canonicalize().unwrap_or_else(|_| target.clone());
        if !canonical_target.starts_with(&canonical_base) {
            return Err("PathEscapeError: sub_path resolves outside of mods_path".to_string());
        }
    }

    log::info!("Scanning filesystem for mods at {}", target.display());

    let mut folders = scan_fs_folders(&target, &base, sub_path.as_deref()).await?;

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
        let base_key = canonical_name_key(&normalize_display_name(&f.folder_name));
        groups.entry(base_key).or_default().push(i);
    }

    let mut conflicts: Vec<ConflictGroup> = Vec::new();
    for (base_key, indices) in &groups {
        if indices.len() < 2 {
            continue;
        }

        // Skip expected DISABLED/ENABLED pairs — these are created by the
        // PrivacyManager mode toggle (adding/removing "DISABLED " prefix).
        // They are NOT user-caused naming conflicts.
        if indices.len() == 2 {
            let has_enabled = indices.iter().any(|&i| folders[i].is_enabled);
            let has_disabled = indices.iter().any(|&i| !folders[i].is_enabled);
            if has_enabled && has_disabled {
                continue;
            }
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
                // Skip expected DISABLED/ENABLED pairs — these are created by the
                // PrivacyManager mode toggle (adding/removing "DISABLED " prefix).
                // One is enabled, the other disabled — this is NOT a user-caused conflict.
                let sibling_disabled = is_disabled_folder(&sibling_name);
                if self_disabled != sibling_disabled {
                    // Expected pair: one DISABLED, one not. Not a real conflict.
                    // Fall through without adding to conflicts.
                } else {
                    // Found a REAL sibling conflict (same prefix state).
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    parent_dir.to_string_lossy().hash(&mut hasher);
                    canonical_name_key(&self_base).hash(&mut hasher);
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
    }

    let (self_node_type, self_classification_reasons, _) =
        crate::services::explorer::classifier::classify_folder(&target);
    let self_is_mod = self_node_type
        == crate::services::explorer::classifier::NodeType::FlatModRoot
        || self_node_type == crate::services::explorer::classifier::NodeType::ModPackRoot
        || self_node_type == crate::services::explorer::classifier::NodeType::VariantContainer;

    // Determine self_is_enabled based on the final path directory component prefix
    let self_is_enabled = if sub_path.as_ref().is_some_and(|sp| !sp.is_empty()) {
        let name = path_file_name_lossy(&target).unwrap_or_default();
        !is_disabled_folder(&name)
    } else {
        true
    };

    let ancestor_info = sub_path.as_deref().and_then(|sp| {
        if sp.is_empty() {
            None
        } else {
            find_disabled_ancestor(&mods_path, sp)
        }
    });

    let (mut ancestor_disabled_by, mut ancestor_disabled_path) = match ancestor_info {
        Some((name, path)) => (Some(name), Some(path)),
        None => (None, None),
    };

    // If the root itself is disabled, treat it as the "ultimate" ancestor lock
    if is_root_disabled && ancestor_disabled_by.is_none() {
        ancestor_disabled_by = Some(
            path_file_name_lossy(&base)
                .map(|n| normalize_display_name(&n))
                .unwrap_or_else(|| "Mods".to_string()),
        );
        ancestor_disabled_path = Some(base.to_string_lossy().to_string());
    }

    Ok(crate::services::explorer::types::FolderGridResponse {
        self_node_type: Some(self_node_type.as_str().to_string()),
        self_is_mod,
        self_is_enabled,
        self_owner_object_id: None,
        self_owner_object_folder_path: None,
        self_classification_reasons,
        children: folders,
        conflicts,
        ancestor_disabled_by,
        ancestor_disabled_path,
    })
}

#[cfg(test)]
#[path = "tests/listing_tests.rs"]
mod tests;
