use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

use super::helpers::{ensure_game_exists, ensure_object_exists, generate_stable_id};
use super::types::{ConfirmedScanItem, SyncResult};
use crate::services::corridor_constants::{
    CORRIDOR_SOURCE_AUTO_TAGGED, CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN,
    DISABLED_REASON_USER,
};

/// Phase 2: Commit user-confirmed scan results to DB (Two-Phase Diffing).
#[allow(clippy::too_many_arguments)]
pub async fn commit_scan_results(
    pool: &SqlitePool,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
    items: Vec<ConfirmedScanItem>,
    resource_dir: Option<&Path>,
    safe_mode_keywords: &[String],
    preserve_existing_mappings: bool,
) -> Result<SyncResult, String> {
    let _ = resource_dir; // reserved for future thumbnail resolution
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    ensure_game_exists(&mut tx, game_id, game_name, game_type, mods_path).await?;

    // Phase 0: Pre-process disk items to resolve `move_from_temp` paths
    let mut disk_entries = Vec::new();
    for item in items {
        if item.skip {
            continue;
        }

        let mut actual_folder_path = item.folder_path.clone();
        if item.move_from_temp {
            let obj_name = item.matched_object.as_deref().unwrap_or("Uncategorized");
            let source_path = Path::new(&item.folder_path);
            if let Some(folder_name) = source_path.file_name() {
                let target_dir = Path::new(mods_path).join(obj_name);
                let target_path = target_dir.join(folder_name);

                if source_path.exists() {
                    if !target_dir.exists() {
                        let _ = std::fs::create_dir_all(&target_dir);
                    }
                    if target_path.exists() {
                        return Err(format!("DUPLICATE|{}", target_path.to_string_lossy()));
                    }
                    if let Err(e) = std::fs::rename(source_path, &target_path) {
                        return Err(format!("Failed to move temp folder: {}", e));
                    } else {
                        actual_folder_path = target_path.to_string_lossy().into_owned();
                    }
                } else {
                    return Err(format!(
                        "Source path does not exist: {}",
                        source_path.display()
                    ));
                }
            } else {
                return Err("Invalid folder path for move_from_temp".to_string());
            }
        }
        disk_entries.push((item, actual_folder_path));
    }

    let total = disk_entries.len();

    // Fetch snapshot of DB state
    let db_mods = crate::database::mod_repo::get_all_mods_sync_info_tx(&mut tx, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut disk_to_db: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut db_matched: HashSet<usize> = HashSet::new();

    fn clean_folder(name: &str) -> String {
        crate::services::scanner::core::normalizer::normalize_display_name(name)
    }

    fn get_parent_and_name(path_str: &str) -> (String, String) {
        let p = Path::new(path_str);
        let parent = p
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        (parent, name)
    }

    // Phase 1: Heuristic Linking
    // Pass A: Exact Match (folder_path == folder_path)
    for disk_idx in 0..disk_entries.len() {
        if disk_to_db.contains_key(&disk_idx) {
            continue;
        }
        for db_idx in 0..db_mods.len() {
            if db_matched.contains(&db_idx) {
                continue;
            }
            if db_mods[db_idx].1 == disk_entries[disk_idx].1 {
                disk_to_db.insert(disk_idx, db_idx);
                db_matched.insert(db_idx);
                break;
            }
        }
    }

    // Pass B: Toggle Match (ignore "DISABLED " prefix)
    for disk_idx in 0..disk_entries.len() {
        if disk_to_db.contains_key(&disk_idx) {
            continue;
        }
        let (disk_parent, disk_name) = get_parent_and_name(&disk_entries[disk_idx].1);
        let disk_clean = clean_folder(&disk_name);

        for db_idx in 0..db_mods.len() {
            if db_matched.contains(&db_idx) {
                continue;
            }
            let (db_parent, db_name) = get_parent_and_name(&db_mods[db_idx].1);
            let db_clean = clean_folder(&db_name);

            if disk_parent == db_parent && disk_clean == db_clean {
                disk_to_db.insert(disk_idx, db_idx);
                db_matched.insert(db_idx);
                break;
            }
        }
    }

    // Pass C: 1:1 Rename Match (isolated unmatched item in same parent directory)
    let mut unmatched_disk_by_parent: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for disk_idx in 0..disk_entries.len() {
        if disk_to_db.contains_key(&disk_idx) {
            continue;
        }
        let (parent, _) = get_parent_and_name(&disk_entries[disk_idx].1);
        unmatched_disk_by_parent
            .entry(parent)
            .or_default()
            .push(disk_idx);
    }

    let mut unmatched_db_by_parent: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for db_idx in 0..db_mods.len() {
        if db_matched.contains(&db_idx) {
            continue;
        }
        let (parent, _) = get_parent_and_name(&db_mods[db_idx].1);
        // Ensure this DB mod isn't physically on disk anymore before considering it renamed
        if !Path::new(&db_mods[db_idx].1).exists() {
            unmatched_db_by_parent
                .entry(parent)
                .or_default()
                .push(db_idx);
        }
    }

    for (parent, disk_indices) in unmatched_disk_by_parent.iter() {
        if disk_indices.len() == 1 {
            if let Some(db_indices) = unmatched_db_by_parent.get(parent) {
                if db_indices.len() == 1 {
                    let disk_idx = disk_indices[0];
                    let db_idx = db_indices[0];
                    disk_to_db.insert(disk_idx, db_idx);
                    db_matched.insert(db_idx);
                }
            }
        }
    }

    // Phase 2: Execution
    let mut new_mods_count = 0;
    let mut updated_mods_count = 0;
    let mut new_objects_count = 0;

    for (disk_idx, (item, actual_folder_path)) in disk_entries.into_iter().enumerate() {
        let current_status = if item.is_disabled {
            "DISABLED"
        } else {
            "ENABLED"
        };

        let (mod_id, existing_object_id) = if let Some(&db_idx) = disk_to_db.get(&disk_idx) {
            let db_mod = &db_mods[db_idx];
            let id = db_mod.0.clone();
            let existing_corridor_source = db_mod.5.as_deref().unwrap_or(CORRIDOR_SOURCE_UNKNOWN);
            let (auto_safe, auto_source) =
                classify_corridor(&item.display_name, safe_mode_keywords);
            let (next_is_safe, next_corridor_source) =
                if existing_corridor_source == CORRIDOR_SOURCE_MANUAL {
                    (db_mod.4, existing_corridor_source)
                } else {
                    (auto_safe, auto_source)
                };

            let path_changed = db_mod.1 != actual_folder_path;
            let status_changed = db_mod.2 != current_status;
            let safety_changed =
                db_mod.4 != next_is_safe || existing_corridor_source != next_corridor_source;

            if path_changed || status_changed || safety_changed {
                let _reason = if item.is_disabled {
                    Some(DISABLED_REASON_USER)
                } else {
                    None
                };
                crate::database::mod_repo::update_mod_identity_tx(
                    &mut tx,
                    &id,
                    &actual_folder_path,
                    &item.display_name,
                    current_status,
                    next_is_safe,
                    next_corridor_source,
                    &db_mod.1, // old path
                    game_id,
                    Some(mods_path),
                )
                .await
                .map_err(|e| e.to_string())?;
                updated_mods_count += 1;
            }
            (id, db_mod.3.clone())
        } else {
            let id = generate_stable_id(game_id, &actual_folder_path);
            let object_type = item.object_type.as_deref().unwrap_or("Other");
            let reason = if item.is_disabled {
                Some(DISABLED_REASON_USER)
            } else {
                None
            };
            let (is_safe, corridor_source) =
                classify_corridor(&item.display_name, safe_mode_keywords);
            crate::database::mod_repo::insert_mod_with_reason_tx(
                &mut tx,
                &id,
                game_id,
                &item.display_name,
                &actual_folder_path,
                Some(mods_path),
                current_status,
                object_type,
                false,
                is_safe,
                corridor_source,
                reason,
            )
            .await
            .map_err(|e| e.to_string())?;
            new_mods_count += 1;
            (id, None)
        };

        // Skip object mutation if preserve_existing_mappings is true and the mod already has an object
        if !(preserve_existing_mappings && existing_object_id.is_some()) {
            let obj_type = item.object_type.as_deref().unwrap_or("Other");
            let (obj_name, db_thumb, tags, meta) = if let Some(ref matched_name) =
                item.matched_object
            {
                (
                    matched_name.to_string(),
                    item.thumbnail_path.as_deref(),
                    item.tags_json.as_deref().unwrap_or("[]"),
                    item.metadata_json.as_deref().unwrap_or("{}"),
                )
            } else {
                let folder_name = Path::new(&actual_folder_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let clean_name = crate::services::scanner::core::normalizer::normalize_display_name(
                    &folder_name,
                );
                (clean_name, None, "[]", "{}")
            };

            let object_folder_path = if item.move_from_temp {
                obj_name.clone()
            } else {
                let actual_path = Path::new(&actual_folder_path);
                let mods_dir = Path::new(mods_path);
                if let Ok(rel_path) = actual_path.strip_prefix(mods_dir) {
                    if let Some(parent) = rel_path.parent() {
                        let parent_str = parent.to_string_lossy().to_string();
                        if parent_str.is_empty() {
                            rel_path.to_string_lossy().to_string()
                        } else {
                            parent_str
                        }
                    } else {
                        rel_path.to_string_lossy().to_string()
                    }
                } else {
                    obj_name.clone()
                }
            };

            let object_id = ensure_object_exists(
                &mut tx,
                game_id,
                &object_folder_path,
                &obj_name,
                obj_type,
                db_thumb,
                tags,
                meta,
                &mut new_objects_count,
            )
            .await?;

            crate::database::mod_repo::update_mod_object_id_and_type_tx(
                &mut tx, &mod_id, &object_id, obj_type,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        let (is_safe, _) = classify_corridor(&item.display_name, safe_mode_keywords);
        if !is_safe {
            let update = crate::services::mods::info_json::ModInfoUpdate {
                is_safe: Some(false),
                ..Default::default()
            };
            let path = std::path::Path::new(&actual_folder_path);
            let _ = crate::services::mods::info_json::update_info_json(path, &update);
        }
    }

    // Phase 3: Purge Deletions
    let mut deleted_mods_count = 0;
    for (db_idx, db_mod) in db_mods.iter().enumerate() {
        if !db_matched.contains(&db_idx) {
            // Only delete if the physical path is truly gone
            if !Path::new(&db_mod.1).exists() {
                crate::database::mod_repo::delete_mod_tx(&mut tx, &db_mod.0)
                    .await
                    .map_err(|e| e.to_string())?;
                deleted_mods_count += 1;
            }
        }
    }

    crate::database::object_repo::delete_ghost_objects_gc(&mut tx, game_id)
        .await
        .map_err(|e| format!("Failed to clean ghost objects: {}", e))?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let temp_dir_path = std::path::Path::new(mods_path).join(".emmm2_temp");
    if temp_dir_path.exists() {
        let _ = std::fs::remove_dir(&temp_dir_path);
    }

    Ok(SyncResult {
        total_scanned: total,
        new_mods: new_mods_count,
        updated_mods: updated_mods_count,
        deleted_mods: deleted_mods_count,
        new_objects: new_objects_count,
    })
}
fn classify_corridor(display_name: &str, safe_mode_keywords: &[String]) -> (bool, &'static str) {
    let folder_name_lower = display_name.to_lowercase();
    let keyword_match = safe_mode_keywords
        .iter()
        .any(|kw| folder_name_lower.contains(&kw.to_lowercase()));

    if keyword_match {
        return (false, CORRIDOR_SOURCE_AUTO_TAGGED);
    }

    (true, CORRIDOR_SOURCE_UNKNOWN)
}
