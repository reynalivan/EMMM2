use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::path::Path;

use super::helpers::{ensure_game_exists, ensure_object_exists, generate_stable_id};
use super::types::{ConfirmedScanItem, SyncResult};

/// Phase 2: Commit user-confirmed scan results to DB.
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
            match source_path.file_name() {
                Some(folder_name) => {
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
                    }
                }
                _ => (),
            }
        }

        let current_status = if item.is_disabled {
            "DISABLED"
        } else {
            "ENABLED"
        };

        let existing = sqlx::query(
            "SELECT id, object_id, status FROM mods WHERE folder_path = ? AND game_id = ?",
        )
        .bind(&actual_folder_path)
        .bind(game_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let mod_id = if let Some(row) = existing {
            let id: String = row.try_get("id").map_err(|e| e.to_string())?;
            let db_status: String = row.try_get("status").map_err(|e| e.to_string())?;
            if db_status != current_status {
                sqlx::query("UPDATE mods SET status = ? WHERE id = ?")
                    .bind(current_status)
                    .bind(&id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                updated_mods_count += 1;
            }
            id
        } else {
            let id = generate_stable_id(game_id, &actual_folder_path);
            let object_type = item.object_type.as_deref().unwrap_or("Other");
            sqlx::query(
                "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_type, is_favorite) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id)
            .bind(game_id)
            .bind(&item.display_name)
            .bind(&actual_folder_path)
            .bind(current_status)
            .bind(object_type)
            .bind(false)
            .execute(&mut *tx)
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
            let clean_name = folder_name
                .strip_prefix(crate::DISABLED_PREFIX)
                .unwrap_or(&folder_name)
                .to_string();
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

        sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
            .bind(&object_id)
            .bind(obj_type)
            .bind(&mod_id)
            .execute(&mut *tx)
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
            sqlx::query("UPDATE objects SET is_safe = 0 WHERE id = ?")
                .bind(&object_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

            let update = crate::services::mod_files::info_json::ModInfoUpdate {
                is_safe: Some(false),
                ..Default::default()
            };
            let path = std::path::Path::new(&actual_folder_path);
            let _ = crate::services::mod_files::info_json::update_info_json(path, &update);
        }
    }

    let deleted_mods_count = handle_deletions(&mut tx, game_id, &processed_paths).await?;

    // Garbage Collector: Delete empty "Ghost" objects left behind after their mod is re-assigned
    // folder_path is an absolute path, so we use LIKE to match the folder basename
    // Also matches DISABLED-prefixed folders (e.g. object "hanya" matches folder "DISABLED hanya")
    sqlx::query(
        "DELETE FROM objects
         WHERE game_id = $1
           AND NOT EXISTS (SELECT 1 FROM mods WHERE object_id = objects.id)
           AND EXISTS (
             SELECT 1 FROM mods
             WHERE (
               folder_path LIKE '%/' || objects.name
               OR folder_path LIKE '%\\' || objects.name
               OR folder_path LIKE '%/DISABLED ' || objects.name
               OR folder_path LIKE '%\\DISABLED ' || objects.name
             ) AND game_id = $1
           )",
    )
    .bind(game_id)
    .execute(&mut *tx)
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
    let all_mods = sqlx::query("SELECT id, folder_path FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_mods_count = 0;
    for row in all_mods {
        let fp: String = row.try_get("folder_path").map_err(|e| e.to_string())?;
        if !processed_paths.contains(&fp) && !Path::new(&fp).exists() {
            let id: String = row.try_get("id").map_err(|e| e.to_string())?;
            sqlx::query("DELETE FROM mods WHERE id = ?")
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
            deleted_mods_count += 1;
        }
    }
    Ok(deleted_mods_count)
}
