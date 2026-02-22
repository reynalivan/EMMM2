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
                    if let Err(e) = std::fs::rename(source_path, &target_path) {
                        log::error!("Failed to move temp folder: {}", e);
                    } else {
                        actual_folder_path = target_path.to_string_lossy().into_owned();
                    }
                }
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
            (folder_name, None, "[]", "{}")
        };

        let object_id = ensure_object_exists(
            &mut tx,
            game_id,
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
    }

    let deleted_mods_count = handle_deletions(&mut tx, game_id, &processed_paths).await?;

    tx.commit().await.map_err(|e| e.to_string())?;

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
