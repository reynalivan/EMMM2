use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

use super::helpers::{
    ensure_game_exists, ensure_object_exists, generate_stable_id,
    resolve_or_create_object_target_for_match,
};
use super::types::{ConfirmedScanItem, SyncResult};
use crate::database::models::ItemStatus;
use crate::repo::{mod_repo, object_repo};
use crate::services::corridor_constants::{
    CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN, DISABLED_REASON_USER,
};

async fn resolve_temp_target_object_folder(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &str,
    physical_name_hint: &str,
    matched_entry_key: Option<&str>,
    object_type: &str,
    db_thumbnail: Option<&str>,
    db_tags_json: &str,
    db_metadata_json: &str,
    db_hash_db_json: Option<&str>,
    db_custom_skins_json: Option<&str>,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    let Some(entry_key) = matched_entry_key else {
        return Ok("Other".to_string());
    };

    let resolved = resolve_or_create_object_target_for_match(
        &mut *conn,
        game_id,
        mods_path,
        physical_name_hint,
        Some(entry_key),
        object_type,
        db_thumbnail,
        db_tags_json,
        db_metadata_json,
        db_hash_db_json,
        db_custom_skins_json,
        new_objects_count,
    )
    .await?;

    Ok(resolved
        .map(|target| target.folder_path)
        .unwrap_or_else(|| "Other".to_string()))
}

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

    let mut collisions = Vec::new();
    let mut new_objects_count = 0;
    // Phase 0: Pre-process disk items to resolve `move_from_temp` paths
    let mut disk_entries = Vec::new();
    for item in items {
        if item.skip {
            continue;
        }

        let mut actual_folder_path = item.folder_path.clone();
        if item.move_from_temp {
            let object_folder = resolve_temp_target_object_folder(
                &mut tx,
                game_id,
                mods_path,
                &item.display_name,
                item.matched_entry_key.as_deref(),
                item.object_type.as_deref().unwrap_or("Other"),
                item.db_thumbnail.as_deref(),
                item.tags_json.as_deref().unwrap_or("[]"),
                item.metadata_json.as_deref().unwrap_or("{}"),
                item.hash_db_json.as_deref(),
                item.custom_skins_json.as_deref(),
                &mut new_objects_count,
            )
            .await?;
            let source_path = Path::new(&item.folder_path);
            if let Some(folder_name) = source_path.file_name() {
                let target_dir = Path::new(mods_path).join(&object_folder);
                let target_path = target_dir.join(folder_name);

                if source_path.exists() {
                    if !target_dir.exists() {
                        let _ = std::fs::create_dir_all(&target_dir);
                    }
                    if target_path.exists() {
                        collisions.push(crate::services::scanner::core::types::CollisionInfo {
                            id: generate_stable_id(game_id, &target_path.to_string_lossy()),
                            source_path: item.folder_path.clone(),
                            target_path: target_path.to_string_lossy().into_owned(),
                            object_name: object_folder.clone(),
                            existing_mod_id: None, // TODO: Look up existing mod ID if mapped
                        });
                        continue;
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
    let db_mods = crate::repo::mod_repo::get_all_mods_sync_info_tx(&mut tx, game_id)
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

    for (disk_idx, (item, actual_folder_path)) in disk_entries.into_iter().enumerate() {
        let current_status = if item.is_disabled {
            ItemStatus::Disabled
        } else {
            ItemStatus::Enabled
        };

        let (mod_id, _current_object_id) = {
            // 1. Identify the Object grouping (depth-1 parent)
            let actual_path = Path::new(&actual_folder_path);
            let mods_dir = Path::new(mods_path);
            let depth_1_folder = if item.move_from_temp {
                String::new() // Handled below
            } else if let Ok(rel_path) = actual_path.strip_prefix(mods_dir) {
                rel_path
                    .components()
                    .next()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let fallback_name = if depth_1_folder.is_empty() {
                Path::new(&actual_folder_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            } else {
                depth_1_folder.clone()
            };

            let obj_name = if !depth_1_folder.is_empty() {
                crate::services::scanner::core::normalizer::normalize_display_name(&depth_1_folder)
            } else {
                crate::services::scanner::core::normalizer::normalize_display_name(&fallback_name)
            };

            let object_folder_path = if item.move_from_temp {
                obj_name.clone()
            } else if !depth_1_folder.is_empty() {
                depth_1_folder.clone()
            } else {
                fallback_name.clone()
            };

            // STRICT DEPTH-1 OBJECT BOUNDARY:
            // Regardless of fuzzy matched_object, the object's core identity (name and folder)
            // is strictly bound to the physical depth-1 directory it lives in.
            // This prevents "ambercn" from merging into "amber".
            // We only keep tags and meta from the match, not the name.
            let final_obj_name = obj_name;

            let obj_type = item.object_type.as_deref().unwrap_or("Other");
            let object_id = ensure_object_exists(
                &mut tx,
                game_id,
                &object_folder_path,
                &final_obj_name,
                obj_type,
                item.db_thumbnail.as_deref(),
                item.tags_json.as_deref().unwrap_or("[]"),
                item.metadata_json.as_deref().unwrap_or("{}"),
                item.hash_db_json.as_deref(),
                item.custom_skins_json.as_deref(),
                &mut new_objects_count,
            )
            .await?;
            object_repo::apply_canonical_match(
                &mut *tx,
                &object_id,
                item.matched_entry_key.as_deref(),
                item.matched_alias_name.as_deref(),
                item.matched_confidence,
                item.matched_reason.as_deref(),
                item.matched_entry_key.as_ref().map(|_| "deepmatch_scanner"),
            )
            .await
            .map_err(|e| e.to_string())?;

            if let Some(&db_idx) = disk_to_db.get(&disk_idx) {
                let db_mod = &db_mods[db_idx];
                let id = db_mod.0.clone();
                let existing_corridor_source =
                    db_mod.5.as_deref().unwrap_or(CORRIDOR_SOURCE_UNKNOWN);
                let (auto_safe, auto_source) =
                    super::helpers::classify_corridor(&item.display_name, safe_mode_keywords);
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
                let object_changed = db_mod.3.as_deref() != Some(&object_id);

                if path_changed || status_changed || safety_changed || object_changed {
                    sqlx::query(
                        "UPDATE mods
                         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, is_safe = ?, corridor_source = ?, disabled_reason = ?, object_id = ?, object_type = ?
                         WHERE folder_path_key = ? AND game_id = ?"
                    )
                    .bind(&id)
                    .bind(&actual_folder_path)
                    .bind(crate::services::path_key::folder_path_key(&actual_folder_path, Some(mods_path)))
                    .bind(&item.display_name)
                    .bind(current_status)
                    .bind(next_is_safe)
                    .bind(next_corridor_source)
                    .bind(if item.is_disabled { Some(DISABLED_REASON_USER) } else { None })
                    .bind(&object_id)
                    .bind(obj_type)
                    .bind(crate::services::path_key::folder_path_key(&db_mod.1, Some(mods_path)))
                    .bind(game_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;

                    updated_mods_count += 1;
                }
                (id, object_id)
            } else {
                let id = generate_stable_id(game_id, &actual_folder_path);
                let (is_safe, corridor_source) =
                    super::helpers::classify_corridor(&item.display_name, safe_mode_keywords);
                mod_repo::insert_mod_with_reason_tx(
                    &mut tx,
                    &id,
                    game_id,
                    &object_id,
                    &item.display_name,
                    &actual_folder_path,
                    Some(mods_path),
                    current_status,
                    obj_type,
                    false,
                    is_safe,
                    corridor_source,
                    if item.is_disabled {
                        Some(DISABLED_REASON_USER)
                    } else {
                        None
                    },
                )
                .await
                .map_err(|e| e.to_string())?;
                new_mods_count += 1;
                (id, object_id)
            }
        };

        // Skip object mutation if preserve_existing_mappings is true and the mod already has an object
        // Wait, we already computed object_id above. We just need to check if we should skip registration.
        if !(preserve_existing_mappings
            && disk_to_db
                .get(&disk_idx)
                .and_then(|&i| db_mods[i].3.as_ref())
                .is_some())
        {
            let actual_path = Path::new(&actual_folder_path);
            let mods_dir = Path::new(mods_path);

            // CRITICAL FIX: Only register as a mod if it's NOT just an empty VariantContainer acting as the Object folder
            let depth = actual_path
                .strip_prefix(mods_dir)
                .map(|p| p.components().count())
                .unwrap_or(0);
            let is_pure_object_container = depth == 1
                && crate::services::explorer::classifier::classify_folder(actual_path).0
                    == crate::services::explorer::classifier::NodeType::VariantContainer;

            if is_pure_object_container {
                // It's a pure container. We ensured the object exists, but we MUST DELETE
                // the mod_id that was just created for it!
                crate::repo::mod_repo::delete_mod_tx(&mut tx, &mod_id)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        let (is_safe, _) =
            super::helpers::classify_corridor(&item.display_name, safe_mode_keywords);
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
                crate::repo::mod_repo::delete_mod_tx(&mut tx, &db_mod.0)
                    .await
                    .map_err(|e| e.to_string())?;
                deleted_mods_count += 1;
            }
        }
    }

    crate::repo::object_repo::delete_ghost_objects_gc(&mut tx, game_id)
        .await
        .map_err(|e| format!("Failed to clean ghost objects: {}", e))?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let temp_dir_path = std::path::Path::new(mods_path).join(".emmm_temp");
    if temp_dir_path.exists() {
        let _ = std::fs::remove_dir(&temp_dir_path);
    }

    Ok(SyncResult {
        total_scanned: total,
        new_mods: new_mods_count,
        updated_mods: updated_mods_count,
        deleted_mods: deleted_mods_count,
        new_objects: new_objects_count,
        collisions,
    })
}
