//! Walk the filesystem to discover nested mod folders for collection save/apply.
//!
//! The scanner only indexes immediate children of `mods_path` into the `mods` table.
//! But users may organize mods inside object subfolders (e.g. `mods_path/Barbara/BarbaraGyaruALL`).
//! These nested mods are invisible to the DB-based collection system.
//!
//! This module bridges that gap by walking the filesystem recursively to find all
//! mod-type folders (`ModPackRoot`, `VariantContainer`, `FlatModRoot`) at any depth,
//! returning their path, display name, enabled state, and safe/unsafe classification.

use serde::Serialize;
use std::path::Path;

use crate::services::explorer::classifier::{classify_folder, NodeType};
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};

/// A nested mod discovered by walking the filesystem.
#[derive(Debug, Clone, Serialize)]
pub struct NestedModState {
    /// Absolute path to the mod folder.
    pub folder_path: String,
    /// Clean display name (DISABLED prefix stripped).
    pub display_name: String,
    /// Whether the folder is enabled (no DISABLED prefix).
    pub is_enabled: bool,
    /// Safe mode flag from info.json (defaults to true if missing).
    pub is_safe: bool,
    /// Parent object folder name (first path segment relative to mods_path).
    pub object_name: Option<String>,
    /// Classified node type (ModPackRoot, VariantContainer, FlatModRoot).
    pub node_type: String,
}

/// Walk all folders under `mods_path` recursively (up to depth 3) and return
/// nested mod folders that are NOT immediate children of `mods_path`.
///
/// Immediate children are already in the `mods` table — this function only
/// returns folders at depth ≥ 2 that are classified as actual mods.
pub fn walk_nested_mods(mods_path: &str) -> Result<Vec<NestedModState>, String> {
    let root = Path::new(mods_path);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    // Read immediate children (depth-1) — these are the "top-level" folders.
    // For each that is a ContainerFolder, recurse into it to find nested mods.
    let top_entries =
        std::fs::read_dir(root).map_err(|e| format!("Failed to read mods directory: {e}"))?;

    for entry in top_entries.flatten() {
        let top_path = entry.path();
        if !top_path.is_dir() {
            continue;
        }

        let top_name = match top_path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        // Skip hidden folders
        if top_name.starts_with('.') {
            continue;
        }

        // Classify top-level folder
        let (top_type, _) = classify_folder(&top_path);

        // Only recurse into ContainerFolders — these are the object/navigation folders
        // that contain actual mod subfolders.
        // ModPackRoot/VariantContainer/FlatModRoot at depth-1 are already in `mods` table.
        if top_type != NodeType::ContainerFolder {
            continue;
        }

        let object_name = normalize_display_name(&top_name);

        // Walk children of this container (depth-2+)
        walk_children(&top_path, &object_name, &mut results, 3)?;
    }

    Ok(results)
}

/// Recursively walk children of a ContainerFolder to find nested mods.
fn walk_children(
    parent: &Path,
    object_name: &str,
    results: &mut Vec<NestedModState>,
    remaining_depth: usize,
) -> Result<(), String> {
    if remaining_depth == 0 {
        return Ok(());
    }

    let entries = match std::fs::read_dir(parent) {
        Ok(e) => e,
        Err(_) => return Ok(()), // silently skip unreadable dirs
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        // Skip hidden folders
        if folder_name.starts_with('.') {
            continue;
        }

        let (node_type, _) = classify_folder(&path);

        match node_type {
            NodeType::ModPackRoot | NodeType::VariantContainer | NodeType::FlatModRoot => {
                let is_enabled = !is_disabled_folder(&folder_name);
                let display_name = normalize_display_name(&folder_name);
                let is_safe = read_is_safe(&path);

                results.push(NestedModState {
                    folder_path: path.to_string_lossy().to_string(),
                    display_name,
                    is_enabled,
                    is_safe,
                    object_name: Some(object_name.to_string()),
                    node_type: node_type.as_str().to_string(),
                });
            }
            NodeType::ContainerFolder => {
                // Recurse deeper into container folders
                walk_children(&path, object_name, results, remaining_depth - 1)?;
            }
            NodeType::InternalAssets => {
                // Skip — these are referenced by a parent mod's ini
            }
        }
    }

    Ok(())
}

/// Read `is_safe` from `info.json` in the folder. Defaults to `true` if absent.
fn read_is_safe(path: &Path) -> bool {
    match crate::services::mods::info_json::read_info_json(path) {
        Ok(Some(info)) => info.is_safe,
        _ => true,
    }
}

/// Generate a deterministic synthetic ID for a nested mod path.
/// Used as the `mod_id` in `collection_items` for nested mods without DB entries.
pub fn nested_mod_id(folder_path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    folder_path.hash(&mut hasher);
    format!("nested_{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_walk_nested_mods_empty_or_missing() {
        assert!(walk_nested_mods("/non/existent/path/for/test")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_walk_nested_mods_structure() {
        let tmp = TempDir::new().unwrap();
        let mods_path = tmp.path();

        // 1. Top-level mod (should NOT be returned, as it's at depth 1)
        let top_mod = mods_path.join("TopMod");
        fs::create_dir(&top_mod).unwrap();
        fs::write(top_mod.join("mod.ini"), "[TextureOverride]").unwrap();

        // 2. Container folder
        let container = mods_path.join("Character");
        fs::create_dir(&container).unwrap();

        // 2a. Nested mod (enabled)
        let nested_enabled = container.join("NestedEnabled");
        fs::create_dir(&nested_enabled).unwrap();
        fs::write(nested_enabled.join("mod.ini"), "[TextureOverride]").unwrap();

        // 2b. Nested mod (disabled)
        let nested_disabled = container.join("DISABLED NestedDisabled");
        fs::create_dir(&nested_disabled).unwrap();
        fs::write(nested_disabled.join("mod.ini"), "[TextureOverride]").unwrap();

        // 3. Deep container folder
        let deep_container = container.join("Deep");
        fs::create_dir(&deep_container).unwrap();

        let deep_mod = deep_container.join("DeepMod");
        fs::create_dir(&deep_mod).unwrap();
        fs::write(deep_mod.join("mod.ini"), "[TextureOverride]").unwrap();

        let results = walk_nested_mods(&mods_path.to_string_lossy()).unwrap();

        assert_eq!(results.len(), 3, "Should find 3 nested mods");

        let enabled_mod = results
            .iter()
            .find(|m| m.display_name == "NestedEnabled")
            .unwrap();
        assert!(enabled_mod.is_enabled);
        assert_eq!(enabled_mod.object_name.as_deref(), Some("Character"));
        assert_eq!(enabled_mod.node_type, "FlatModRoot");

        let disabled_mod = results
            .iter()
            .find(|m| m.display_name == "NestedDisabled")
            .unwrap();
        assert!(!disabled_mod.is_enabled);
        assert_eq!(disabled_mod.object_name.as_deref(), Some("Character"));

        let deep = results
            .iter()
            .find(|m| m.display_name == "DeepMod")
            .unwrap();
        assert!(deep.is_enabled);
        assert_eq!(deep.object_name.as_deref(), Some("Character"));
    }
}
