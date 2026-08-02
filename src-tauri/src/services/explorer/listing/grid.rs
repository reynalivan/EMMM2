use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::common::normalizer::{is_disabled_folder, normalize_display_name};
use crate::common::path_key::{canonical_name_key, names_equal_by_key, path_file_name_lossy};
use crate::services::explorer::types::ConflictMember;

use super::scan::{find_disabled_ancestor, scan_fs_folders};

/// Case-insensitive child lookup: some volumes resolve paths case-sensitively
/// even when the filesystem claims otherwise.
fn find_child_by_name_key(parent: &Path, needle: &str) -> Option<PathBuf> {
    std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .find(|entry| names_equal_by_key(&entry.file_name().to_string_lossy(), needle))
        .map(|entry| entry.path())
}

/// Stable id for a conflict group, derived from where it lives plus its
/// normalized base name so the same clash keeps the same id across listings.
fn conflict_group_id(directory: &Path, base_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    directory.to_string_lossy().hash(&mut hasher);
    base_key.hash(&mut hasher);
    format!("cg_{:016x}", hasher.finish())
}

fn conflict_member_from_disk(path: &Path, folder_name: String, is_enabled: bool) -> ConflictMember {
    let metadata = std::fs::metadata(path).ok();
    ConflictMember {
        path: path.to_string_lossy().to_string(),
        folder_name,
        is_enabled,
        modified_at: metadata
            .as_ref()
            .and_then(|value| value.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        size_bytes: metadata.as_ref().map(|value| value.len()).unwrap_or(0),
    }
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
    if let Some(sp) = &sub_path {
        let requested = Path::new(sp);
        if requested.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            return Err("PathEscapeError: sub_path resolves outside of mods_path".to_string());
        }
    }

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
        let disabled_needle = format!("{}{}", crate::DISABLED_PREFIX, needle);
        find_child_by_name_key(parent, &needle)
            .or_else(|| find_child_by_name_key(parent, &disabled_needle))
            .unwrap_or(target)
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
        let canonical_target = if target.exists() {
            target.canonicalize().unwrap_or_else(|_| target.clone())
        } else if let Some(sp) = sub_path.as_deref().filter(|value| !value.is_empty()) {
            canonical_base.join(sp)
        } else {
            canonical_base.clone()
        };
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
        let group_id = conflict_group_id(&target, base_key);
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
                    conflicts.push(ConflictGroup {
                        group_id: conflict_group_id(parent_dir, &canonical_name_key(&self_base)),
                        base_name: self_base,
                        members: vec![
                            conflict_member_from_disk(&target, self_name.clone(), !self_disabled),
                            conflict_member_from_disk(
                                &sibling_path,
                                sibling_name.clone(),
                                self_disabled,
                            ),
                        ],
                    });
                }
            }
        }
    }

    let (self_node_type, self_classification_reasons, _) =
        crate::common::classifier::classify_folder(&target);
    let self_is_mod = self_node_type == crate::common::classifier::NodeType::FlatModRoot
        || self_node_type == crate::common::classifier::NodeType::ModPackRoot
        || self_node_type == crate::common::classifier::NodeType::VariantContainer;

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
