use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

use super::helpers::{ensure_game_exists, ensure_object_exists, generate_stable_id};
use super::types::{ConfirmedScanItem, SyncResult};

/// Phase 2: Commit user-confirmed scan results to DB.
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
) -> Result<SyncResult, String> {
    let _ = resource_dir; // reserved for future thumbnail resolution
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    ensure_game_exists(&mut tx, game_id, game_name, game_type, mods_path).await?;

    let total = items.len();
    let mut new_mods_count = 0;
    let mut updated_mods_count = 0;
    let mut new_objects_count = 0;
    let mut processed_paths = HashSet::new();

    for item in &items {
        if item.skip {
            processed_paths.insert(item.folder_path.clone());
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

                    // Check collision
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

        let current_status = if item.is_disabled {
            "DISABLED"
        } else {
            "ENABLED"
        };

        let existing = crate::database::mod_repo::get_mod_id_and_status_by_path(
            &mut *tx,
            &actual_folder_path,
            game_id,
        )
        .await
        .map_err(|e| e.to_string())?;

        let mod_id = if let Some((id, _, db_status)) = existing {
            if db_status != current_status {
                crate::database::mod_repo::update_mod_status_tx(&mut *tx, &id, current_status)
                    .await
                    .map_err(|e| e.to_string())?;
                updated_mods_count += 1;
            }
            id
        } else {
            let id = generate_stable_id(game_id, &actual_folder_path);
            let object_type = item.object_type.as_deref().unwrap_or("Other");
            crate::database::mod_repo::insert_mod_tx(
                &mut *tx,
                &id,
                game_id,
                &item.display_name,
                &actual_folder_path,
                current_status,
                object_type,
                false,
            )
            .await
            .map_err(|e| e.to_string())?;
            new_mods_count += 1;
            id
        };

        processed_paths.insert(item.folder_path.clone());

        let obj_type = item.object_type.as_deref().unwrap_or("Other");
        let (obj_name, db_thumb, tags, meta) = if let Some(ref matched_name) = item.matched_object {
            (
                matched_name.to_string(),
                item.thumbnail_path.as_deref(),
                item.tags_json.as_deref().unwrap_or("[]"),
                item.metadata_json.as_deref().unwrap_or("{}"),
            )
        } else {
            let folder_name = Path::new(&item.folder_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            // Strip DISABLED prefix so we never create "DISABLED xyz" objects
            let clean_name =
                crate::services::scanner::core::normalizer::normalize_display_name(&folder_name);
            (clean_name, None, "[]", "{}")
        };

        // Calculate the physical object folder path relative to mods_path.
        // IMPORTANT: folder_path must always point to an actual filesystem directory,
        // NOT the matched display name (obj_name). When a mod is directly under
        // mods_path/ (parent is empty), use the mod's own folder name so the
        // FolderGrid can resolve the path correctly.
        let object_folder_path = if item.move_from_temp {
            obj_name.clone() // Auto-Organize drops them directly into mods_path/obj_name
        } else {
            let actual_path = Path::new(&actual_folder_path);
            let mods_dir = Path::new(mods_path);
            if let Ok(rel_path) = actual_path.strip_prefix(mods_dir) {
                if let Some(parent) = rel_path.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if parent_str.is_empty() {
                        // Mod is directly in mods_path — use its own folder name
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
            &mut *tx, &mod_id, &object_id, obj_type,
        )
        .await
        .map_err(|e| e.to_string())?;

        // Handle auto-classification
        let folder_name_lower = item.display_name.to_lowercase();
        let mut is_safe = true;
        for kw in safe_mode_keywords {
            if folder_name_lower.contains(&kw.to_lowercase()) {
                is_safe = false;
                break;
            }
        }

        if !is_safe {
            crate::database::object_repo::update_object_is_safe_tx(&mut *tx, &object_id, false)
                .await
                .map_err(|e| e.to_string())?;

            let update = crate::services::mods::info_json::ModInfoUpdate {
                is_safe: Some(false),
                ..Default::default()
            };
            let path = std::path::Path::new(&actual_folder_path);
            let _ = crate::services::mods::info_json::update_info_json(path, &update);
        }
    }

    let deleted_mods_count = handle_deletions(&mut tx, game_id, &processed_paths).await?;

    // Garbage Collector: Delete empty "Ghost" objects left behind after their mod is re-assigned
    // folder_path is an absolute path, so we use LIKE to match the folder basename
    // Also matches DISABLED-prefixed folders (e.g. object "hanya" matches folder "DISABLED hanya")
    // Garbage Collector: Delete empty "Ghost" objects
    // Only delete objects that have NO associated mods.
    // The previous logic incorrectly required a mod with a matching folder name to exist elsewhere.
    crate::database::object_repo::delete_ghost_objects_gc(&mut *tx, game_id)
        .await
        .map_err(|e| format!("Failed to clean ghost objects: {}", e))?;

    tx.commit().await.map_err(|e| e.to_string())?;

    // Attempt to clean up the temp folder if it is empty after moves
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

async fn handle_deletions(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    game_id: &str,
    processed_paths: &HashSet<String>,
) -> Result<usize, String> {
    let all_mods = crate::database::mod_repo::get_all_mods_id_and_paths_tx(&mut **tx, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_mods_count = 0;
    for (id, fp) in all_mods {
        if !processed_paths.contains(&fp) && !Path::new(&fp).exists() {
            crate::database::mod_repo::delete_mod_tx(&mut **tx, &id)
                .await
                .map_err(|e| e.to_string())?;
            deleted_mods_count += 1;
        }
    }
    Ok(deleted_mods_count)
}
