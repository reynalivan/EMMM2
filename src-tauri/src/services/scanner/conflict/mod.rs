//! Shader & buffer conflict detection for 3DMigoto mods.
//!
//! Parses `.ini` files for `[TextureOverride...]` sections with `hash = xxxx`
//! and reports when 2+ mods share the same hash (potential in-game conflict).
//!
//! # Covers: US-2.Z, TC-2.4-01

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub mod detect;

/// Information about a shader/buffer hash conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// The conflicting hash value.
    pub hash: String,
    /// Section name where the hash was found.
    pub section_name: String,
    /// Paths of the mods that conflict.
    pub mod_paths: Vec<String>,
}

/// Entry tracking a single hash occurrence.
#[derive(Debug, Clone)]
struct HashEntry {
    hash: String,
    section: String,
    mod_root: PathBuf,
}

/// Detect shader/buffer conflicts across multiple INI files.
///
/// Scans each `.ini` file for `[TextureOverride...]` sections containing
/// `hash = <value>`, then groups by hash to find conflicts.
///
/// # Returns
/// - A `Vec<ConflictInfo>` for each hash that appears in 2+ different mod paths.
///
/// # Covers: TC-2.4-01
pub fn detect_conflicts(ini_files: &[(PathBuf, PathBuf)]) -> Vec<ConflictInfo> {
    let mut hash_map: HashMap<String, Vec<HashEntry>> = HashMap::new();

    for (mod_root, ini_path) in ini_files {
        let entries = parse_ini_hashes(ini_path, mod_root);
        for entry in entries {
            hash_map.entry(entry.hash.clone()).or_default().push(entry);
        }
    }

    // Find conflicts: hashes with entries from 2+ different mod roots
    hash_map
        .into_iter()
        .filter_map(|(hash, entries)| {
            // Deduplicate by mod root path
            let unique_paths: Vec<String> = entries
                .iter()
                .map(|e| e.mod_root.to_string_lossy().to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if unique_paths.len() < 2 {
                return None;
            }

            let section_name = entries
                .first()
                .map(|e| e.section.clone())
                .unwrap_or_default();

            Some(ConflictInfo {
                hash,
                section_name,
                mod_paths: unique_paths,
            })
        })
        .collect()
}

/// Parse a single INI file for TextureOverride hash entries.
///
/// Looks for sections matching `[TextureOverride...]` and extracts `hash` values.
fn parse_ini_hashes(ini_path: &Path, mod_root: &Path) -> Vec<HashEntry> {
    let content = match fs::read_to_string(ini_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read INI {}: {e}", ini_path.display());
            return Vec::new();
        }
    };

    // Skip empty files (EC-2.05)
    if content.trim().is_empty() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut current_section = String::new();
    let mut in_texture_override = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].to_string();
            in_texture_override = current_section
                .to_lowercase()
                .starts_with("textureoverride");
            continue;
        }

        // Hash value within a TextureOverride section
        if in_texture_override {
            if let Some(hash_val) = parse_hash_line(trimmed) {
                entries.push(HashEntry {
                    hash: hash_val,
                    section: current_section.clone(),
                    mod_root: mod_root.to_path_buf(),
                });
            }
        }
    }

    entries
}

/// Parse a line like `hash = abcd1234` and return the hash value.
fn parse_hash_line(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    if !lower.starts_with("hash") {
        return None;
    }

    // Split on '=' and get the value part
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let key = parts[0].trim().to_lowercase();
    if key != "hash" {
        return None;
    }

    let value = parts[1].trim().to_string();
    if value.is_empty() {
        return None;
    }
    Some(value)
}

#[cfg(test)]
#[path = "tests/conflict_tests.rs"]
mod tests;

/// Info about an enabled duplicate/conflicting mod for a given object.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DuplicateModInfo {
    pub mod_id: String,
    pub folder_path: String,
    pub actual_name: String,
}

/// Find all enabled mods in the same object as `folder_path` (i.e. duplicates/conflicts).
pub async fn get_duplicates_for_mod_service(
    pool: &sqlx::SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Vec<DuplicateModInfo>, String> {
    // Resolve the object_id for the given folder
    let object_id =
        crate::database::mod_repo::get_object_id_by_folder_and_game(pool, folder_path, game_id)
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

    let object_id = match object_id {
        Some(id) => id,
        None => return Ok(vec![]), // No object — no duplicates possible
    };

    let duplicates =
        crate::database::mod_repo::get_enabled_duplicates(pool, &object_id, game_id, folder_path)
            .await
            .map_err(|e| format!("DB duplicate query failed: {e}"))?;

    Ok(duplicates
        .into_iter()
        .map(|(mod_id, path, name)| DuplicateModInfo {
            mod_id,
            folder_path: path,
            actual_name: name,
        })
        .collect())
}

/// Enable a specific mod and disable all other enabled siblings for the same object.
/// Wrapped here to decouple the command layer from direct database queries and orchestration logic.
pub async fn enable_only_this_service(
    pool: &sqlx::SqlitePool,
    state: &tauri::State<'_, crate::services::scanner::watcher::WatcherState>,
    target_path: String,
    game_id: &str,
) -> Result<crate::services::mods::bulk::BulkResult, String> {
    use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
    use crate::services::mods::bulk::{BulkActionError, BulkResult};

    let mut success = Vec::new();
    let mut failures = Vec::new();

    // 1. Find the target mod's object_id from DB
    let target_object_id =
        crate::database::mod_repo::get_object_id_by_folder_and_game(pool, &target_path, game_id)
            .await
            .map_err(|e| format!("DB query failed: {e}"))?;

    let object_id = match target_object_id {
        Some(id) => id,
        None => {
            // No object_id — just enable the target without disabling siblings
            let new_path = toggle_mod_inner(state, target_path, true).await?;
            return Ok(BulkResult {
                success: vec![new_path],
                failures: vec![],
            });
        }
    };

    // 2. Find all other ENABLED mods with the same object_id (siblings)
    let sibling_paths = crate::database::mod_repo::get_enabled_siblings_paths(
        pool,
        &object_id,
        game_id,
        &target_path,
    )
    .await
    .map_err(|e| format!("DB sibling query failed: {e}"))?;

    // 3. Disable all siblings
    for sibling_path in sibling_paths {
        match toggle_mod_inner(state, sibling_path.clone(), false).await {
            Ok(new_path) => success.push(new_path),
            Err(e) => failures.push(BulkActionError {
                path: sibling_path,
                error: e,
            }),
        }
    }

    // 4. Enable the target
    match toggle_mod_inner(state, target_path.clone(), true).await {
        Ok(new_path) => success.push(new_path),
        Err(e) => failures.push(BulkActionError {
            path: target_path,
            error: e,
        }),
    }

    Ok(BulkResult { success, failures })
}
