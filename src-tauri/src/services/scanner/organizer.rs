use super::deep_matcher::{self, MasterDb, MatchLevel};
use super::walker;
use crate::services::file_ops::info_json;
use std::fs;
use std::path::{Path, PathBuf};

/// Result of an organization operation.
pub struct OrganizeResult {
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub object_name: String,
    pub object_type: String,
}

/// Organize a single mod folder based on Deep Matcher.
///
/// 1. Scans content.
/// 2. Runs Deep Matcher.
/// 3. Moves to `{target_root}/{Object_Type}/{Object_Name}/{Mod_Dir_Name}`.
/// 4. Updates `info.json` with matched metadata.
pub fn organize_mod(
    path: &Path,
    target_root: &Path,
    db: &MasterDb,
) -> Result<OrganizeResult, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    // 1. Scan Content
    // We use depth 2 for speed, deeper if needed? Default is 3 in walker.
    let content = walker::scan_folder_content(path, 3);

    // 2. Candidate wrapper
    let folder_name = path
        .file_name()
        .ok_or("Invalid folder name")?
        .to_string_lossy()
        .to_string();

    let candidate = walker::ModCandidate {
        path: path.to_path_buf(),
        raw_name: folder_name.clone(),
        display_name: folder_name.clone(),
        is_disabled: folder_name.starts_with("DISABLED "),
    };

    // 3. Match
    let match_result = deep_matcher::match_folder(&candidate, db, &content);

    // 4. Determine Target
    let (category, obj_name) = if match_result.level == MatchLevel::Unmatched {
        ("Other".to_string(), "Unknown".to_string())
    } else {
        (
            match_result.object_type.clone(),
            match_result.object_name.clone(),
        )
    };

    // Construct path: target_root / Category / ObjectName / ModResultName
    // We sanitize ObjectName just in case
    let safe_obj_name = sanitize_filename::sanitize(&obj_name);
    let safe_category = sanitize_filename::sanitize(&category);

    let target_parent = if match_result.level == MatchLevel::Unmatched {
        target_root.join("Other")
    } else if match_result.object_type.eq_ignore_ascii_case("Character") {
        // Flatten Characters: Mods/Ayaka instead of Mods/Character/Ayaka
        target_root.join(&safe_obj_name)
    } else {
        target_root.join(&safe_category).join(&safe_obj_name)
    };

    // Use skin_folder_name as the destination folder name if present,
    // otherwise use the original folder name.
    // e.g. "JeanCN" skin folder → Jean/JeanCN, "Jean mods" → Jean/Jean mods
    let dest_folder_name = match &match_result.skin_folder_name {
        Some(skin_name) if !skin_name.is_empty() => {
            // Prefix with DISABLED if the mod was disabled
            if candidate.is_disabled {
                format!("DISABLED {skin_name}")
            } else {
                skin_name.clone()
            }
        }
        _ => folder_name.clone(),
    };

    let target_path = target_parent.join(&dest_folder_name);

    // If target is same as source, do nothing
    if target_path == path {
        return Ok(OrganizeResult {
            original_path: path.to_path_buf(),
            new_path: path.to_path_buf(),
            object_name: obj_name,
            object_type: category,
        });
    }

    // Ensure parent exists
    fs::create_dir_all(&target_parent).map_err(|e| format!("Failed to create parent dir: {e}"))?;

    // Check collision
    if target_path.exists() {
        return Err(format!(
            "Destination already exists: {}",
            target_path.display()
        ));
    }

    // Move
    fs::rename(path, &target_path).map_err(|e| format!("Failed to move folder: {e}"))?;

    // Update info.json
    if match_result.level != MatchLevel::Unmatched {
        let mut meta =
            std::collections::HashMap::from([("character".to_string(), obj_name.clone())]);
        meta.insert("category".to_string(), category.clone());

        let update = info_json::ModInfoUpdate {
            // We store the matched object name in metadata for reference
            metadata: Some(meta),
            ..Default::default()
        };
        let _ = info_json::update_info_json(&target_path, &update);
    }

    Ok(OrganizeResult {
        original_path: path.to_path_buf(),
        new_path: target_path,
        object_name: obj_name,
        object_type: category,
    })
}
