//! Phase 0: resolve `move_from_temp` items to their final on-disk location.

use std::path::Path;

use crate::repo::mod_repo;
use crate::repo::stable_ids::generate_stable_id;
use crate::services::scanner::core::types::CollisionInfo;
use crate::services::scanner::sync::helpers::{
    resolve_or_create_object_target_for_match, ResolveObjectTargetInput,
};
use crate::services::scanner::sync::types::ConfirmedScanItem;

use super::request::CommitCtx;

async fn resolve_temp_target_object_folder(
    conn: &mut sqlx::SqliteConnection,
    input: ResolveObjectTargetInput<'_>,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    if input.matched_entry_key.is_none() {
        return Ok("Other".to_string());
    }

    let resolved =
        resolve_or_create_object_target_for_match(&mut *conn, input, new_objects_count).await?;

    Ok(resolved
        .map(|target| target.folder_path)
        .unwrap_or_else(|| "Other".to_string()))
}

/// Moves staged temp folders into their object directory, reporting collisions.
pub(super) async fn prepare_disk_entries(
    tx: &mut sqlx::SqliteConnection,
    ctx: &CommitCtx<'_>,
    items: Vec<ConfirmedScanItem>,
    new_objects_count: &mut usize,
) -> Result<(Vec<(ConfirmedScanItem, String)>, Vec<CollisionInfo>), String> {
    let game_id = ctx.game_id;
    let mods_path = ctx.mods_path;
    let mut collisions = Vec::new();
    let mut disk_entries = Vec::new();
    for item in items {
        if item.skip {
            continue;
        }

        let mut actual_folder_path = item.folder_path.clone();
        if item.move_from_temp {
            let object_thumbnail = item
                .db_thumbnail
                .as_deref()
                .or(item.thumbnail_path.as_deref());
            let object_folder = resolve_temp_target_object_folder(
                &mut *tx,
                ResolveObjectTargetInput {
                    game_id,
                    mods_path,
                    physical_name_hint: &item.display_name,
                    matched_entry_key: item.matched_entry_key.as_deref(),
                    object_type: item.object_type.as_deref().unwrap_or("Other"),
                    db_thumbnail: object_thumbnail,
                    db_tags_json: item.tags_json.as_deref().unwrap_or("[]"),
                    db_metadata_json: item.metadata_json.as_deref().unwrap_or("{}"),
                    db_hash_db_json: item.hash_db_json.as_deref(),
                    db_custom_skins_json: item.custom_skins_json.as_deref(),
                },
                &mut *new_objects_count,
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
                        let target_path_string = target_path.to_string_lossy().into_owned();
                        let existing_mod_id = mod_repo::get_mod_id_by_path_tx(
                            &mut *tx,
                            &target_path_string,
                            game_id,
                            Some(mods_path),
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                        collisions.push(crate::services::scanner::core::types::CollisionInfo {
                            id: generate_stable_id(game_id, &target_path_string),
                            source_path: item.folder_path.clone(),
                            target_path: target_path_string,
                            object_name: object_folder.clone(),
                            existing_mod_id,
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

    Ok((disk_entries, collisions))
}
