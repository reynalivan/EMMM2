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
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ConflictInfo {
    /// The conflicting hash value.
    pub hash: String,
    /// Section name where the hash was found.
    pub section_name: String,
    /// Paths of the mods that conflict.
    pub mod_paths: Vec<String>,
    /// Whether at least two conflicting mods are currently enabled.
    pub is_active: bool,
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
                mod_paths: unique_paths.clone(),
                // Determine if active: 2+ mods must be enabled.
                // Note: We need a way to check status. In this pure-FS service,
                // we check if folder name doesn't start with "DISABLED ".
                is_active: unique_paths
                    .iter()
                    .filter(|p| {
                        !crate::services::scanner::core::normalizer::is_disabled_folder(
                            Path::new(p)
                                .file_name()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap_or(""),
                        )
                    })
                    .count()
                    >= 2,
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
            let section_lower = current_section.to_lowercase();
            in_texture_override = section_lower.starts_with("textureoverride")
                || section_lower.starts_with("shaderoverride")
                || section_lower.starts_with("resource")
                || section_lower.starts_with("customshader");
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

use crate::domain::errors::AppError;

/// Find all enabled mods in the same object as `folder_path` (i.e. duplicates/conflicts).
pub async fn get_duplicates_for_mod_service(
    pool: &sqlx::SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Vec<crate::domain::mods::DuplicateModInfo>, AppError> {
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .unwrap_or_default();

    // Resolve the object_id for the given folder
    let object_id =
        crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, folder_path, game_id)
            .await
            .map_err(|e| AppError::Io(format!("DB query failed: {e}")))?;

    let object_id = match object_id {
        Some(id) => id,
        None => return Ok(vec![]), // No object — no duplicates possible
    };

    let duplicates =
        crate::repo::mod_repo::get_enabled_duplicates(pool, &object_id, game_id, folder_path)
            .await
            .map_err(|e| AppError::Io(format!("DB duplicate query failed: {e}")))?;

    let mut result = Vec::new();
    let mut relevant_mod_ids: Vec<String> = Vec::new();

    for (mod_id, path, name) in duplicates {
        // Variant Detection (Epic 11 Alignment)
        let mut is_variant = false;
        let mut parent_path = String::new();

        if let (Some(target_parent), Some(dup_parent)) =
            (Path::new(folder_path).parent(), Path::new(&path).parent())
        {
            if target_parent == dup_parent {
                let (node_type, _, _) = crate::services::explorer::classifier::classify_folder(
                    &Path::new(&mods_path).join(target_parent),
                );
                if node_type == crate::services::explorer::classifier::NodeType::VariantContainer {
                    is_variant = true;
                    parent_path = target_parent.to_string_lossy().to_string();
                }
            }
        }

        result.push(crate::domain::mods::DuplicateModInfo {
            mod_id: mod_id.clone(),
            object_id: object_id.clone(),
            folder_path: path,
            actual_name: name,
            is_variant,
            parent_path,
        });
        relevant_mod_ids.push(mod_id);
    }

    // Include the target mod ID in the set to check for ignores
    let target_mod_id_search: Result<Option<(String, Option<String>, i64)>, sqlx::Error> =
        crate::repo::mod_repo::get_mod_id_and_status_by_path_any(pool, folder_path, game_id).await;

    let target_mod_id = match target_mod_id_search {
        Ok(Some((id, _, _))) => id,
        _ => String::new(),
    };

    if !target_mod_id.is_empty() {
        relevant_mod_ids.push(target_mod_id);
    }

    // Check if this specific combination is ignored
    let ignored = crate::repo::conflict_repo::is_conflict_ignored(
        pool,
        game_id,
        &object_id,
        &relevant_mod_ids,
    )
    .await
    .unwrap_or(false);

    if ignored {
        return Ok(vec![]);
    }

    Ok(result)
}

/// Enable a specific mod and disable all other enabled siblings for the same object.
/// Wrapped here to decouple the command layer from direct database queries and orchestration logic.
pub async fn enable_only_this_service(
    config: &crate::services::config::ConfigService,
    pool: &sqlx::SqlitePool,
    state: &tauri::State<'_, crate::services::scanner::watcher::WatcherState>,
    target_path: String,
    game_id: &str,
) -> Result<crate::services::mods::bulk::BulkResult, AppError> {
    use crate::services::mods::bulk::{BulkActionError, BulkResult};
    use crate::services::mods::core_ops::toggle_mod_inner;
    use std::path::Path;

    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods path".to_string()))?;

    let target_rel = Path::new(&target_path)
        .strip_prefix(&mods_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| target_path.clone());

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut db_updates = Vec::new();

    let target_object_id =
        crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, &target_rel, game_id)
            .await
            .map_err(|e| AppError::Io(format!("DB query failed: {e}")))?;

    if let Some(object_id) = target_object_id {
        let sibling_paths = crate::repo::mod_repo::get_enabled_siblings_paths(
            pool,
            &object_id,
            game_id,
            &target_rel,
        )
        .await
        .map_err(|e| AppError::Io(format!("DB sibling query failed: {e}")))?;

        for sibling_rel in sibling_paths {
            let sibling_abs = Path::new(&mods_path)
                .join(&sibling_rel)
                .to_string_lossy()
                .to_string();
            match toggle_mod_inner(state, sibling_abs.clone(), false).await {
                Ok(new_abs_path) => {
                    let new_rel = Path::new(&new_abs_path)
                        .strip_prefix(&mods_path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| new_abs_path.clone());

                    db_updates.push((
                        sibling_rel.clone(),
                        new_rel.clone(),
                        crate::database::models::ItemStatus::Disabled,
                    ));
                    success.push(new_abs_path);

                    if sibling_rel != new_rel {
                        let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
                            pool,
                            &sibling_rel,
                            &new_rel,
                            None,
                        )
                        .await;
                    }
                }
                Err(e) => failures.push(BulkActionError {
                    path: sibling_abs,
                    error: e,
                }),
            }
        }
    }

    match toggle_mod_inner(state, target_path.clone(), true).await {
        Ok(new_abs_path) => {
            let new_rel = Path::new(&new_abs_path)
                .strip_prefix(&mods_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| new_abs_path.clone());

            db_updates.push((
                target_rel.clone(),
                new_rel.clone(),
                crate::database::models::ItemStatus::Enabled,
            ));
            success.push(new_abs_path);

            if target_rel != new_rel {
                let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
                    pool,
                    &target_rel,
                    &new_rel,
                    None,
                )
                .await;
            }
        }
        Err(e) => failures.push(BulkActionError {
            path: target_path,
            error: e,
        }),
    }

    if !db_updates.is_empty() {
        if let Err(e) = crate::repo::mod_repo::batch_update_path_and_status(pool, &db_updates).await
        {
            log::error!(
                "Failed batch updating mod paths after enable-only-this: {}",
                e
            );
        }
    }

    let _ = crate::services::corridor_service::recompute_signature(pool, game_id, true).await;
    let _ = crate::services::corridor_service::recompute_signature(pool, game_id, false).await;

    let _ =
        crate::services::runtime_projection_service::rebuild_game_projection(pool, game_id).await;
    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        pool,
        config,
        state.suppressor.clone(),
        game_id,
        &[true, false],
        true,
        true,
    )
    .await;

    Ok(BulkResult { success, failures })
}
