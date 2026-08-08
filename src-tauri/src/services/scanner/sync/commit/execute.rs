//! Phase 2: persist each disk entry as object + mod rows.

use crate::domain::errors::ScannerError;
use std::collections::HashMap;
use std::path::Path;

use crate::common::corridor_constants::{
    CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN, DISABLED_REASON_USER,
};
use crate::domain::models::ItemStatus;
use crate::repo::stable_ids::generate_stable_id;
use crate::repo::{mod_repo, object_repo};
use crate::services::scanner::sync::helpers::ensure_object_exists;
use crate::services::scanner::sync::types::ConfirmedScanItem;

use super::request::{CommitCtx, DbModRow};

/// Returns `(new_mods_count, updated_mods_count)`.
pub(super) async fn execute_entries(
    tx: &mut sqlx::SqliteConnection,
    ctx: &CommitCtx<'_>,
    disk_entries: Vec<(ConfirmedScanItem, String)>,
    db_mods: &[DbModRow],
    disk_to_db: &HashMap<usize, usize>,
    new_objects_count: &mut usize,
) -> Result<(usize, usize), ScannerError> {
    let game_id = ctx.game_id;
    let mods_path = ctx.mods_path;
    let safe_mode_keywords = ctx.safe_mode_keywords;
    let preserve_existing_mappings = ctx.preserve_existing_mappings;
    let mut new_mods_count = 0;
    let mut updated_mods_count = 0;

    for (disk_idx, (item, actual_folder_path)) in disk_entries.into_iter().enumerate() {
        let current_status = if item.is_disabled {
            ItemStatus::Disabled
        } else {
            ItemStatus::Enabled
        };

        // What goes in the column, as opposed to what is used to touch the
        // disk. Disk reconcile stores `folder_path` relative to the mods root
        // and matches rows on that form, so an absolute value here left the
        // row invisible to every one of its three lookups. `actual_folder_path`
        // stays absolute below: it addresses real files, and the stable id is
        // derived from it.
        let stored_folder_path =
            crate::common::path_key::relative_to_root(&actual_folder_path, Path::new(mods_path));

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
                crate::common::normalizer::normalize_display_name(&depth_1_folder).into_owned()
            } else {
                crate::common::normalizer::normalize_display_name(&fallback_name).into_owned()
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
            let object_thumbnail = item
                .db_thumbnail
                .as_deref()
                .or(item.thumbnail_path.as_deref());
            let object_id = ensure_object_exists(
                &mut *tx,
                crate::domain::objects::EnsureObjectInput {
                    game_id,
                    folder_path: &object_folder_path,
                    obj_name: &final_obj_name,
                    obj_type,
                    // The canonical relation, not the thumbnail: the old
                    // proxy also fired on a preview image found on disk.
                    source: if item.matched_entry_key.is_some() {
                        crate::domain::objects::MatchSource::MasterDb
                    } else {
                        crate::domain::objects::MatchSource::Disk
                    },
                    db_thumbnail: object_thumbnail,
                    db_tags_json: item.tags_json.as_deref().unwrap_or("[]"),
                    db_metadata_json: item.metadata_json.as_deref().unwrap_or("{}"),
                    db_hash_db_json: item.hash_db_json.as_deref(),
                    db_custom_skins_json: item.custom_skins_json.as_deref(),
                },
                &mut *new_objects_count,
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
            .await?;

            if let Some(&db_idx) = disk_to_db.get(&disk_idx) {
                let db_mod = &db_mods[db_idx];
                let id = db_mod.0.clone();
                let existing_corridor_source =
                    db_mod.5.as_deref().unwrap_or(CORRIDOR_SOURCE_UNKNOWN);
                let (auto_safe, auto_source) =
                    crate::services::scanner::sync::helpers::classify_corridor(
                        &item.display_name,
                        safe_mode_keywords,
                    );
                let (next_is_safe, next_corridor_source) =
                    if existing_corridor_source == CORRIDOR_SOURCE_MANUAL {
                        (db_mod.4, existing_corridor_source)
                    } else {
                        (auto_safe, auto_source)
                    };

                let path_changed = db_mod.1 != stored_folder_path;
                let status_changed = db_mod.2 != current_status;
                let safety_changed =
                    db_mod.4 != next_is_safe || existing_corridor_source != next_corridor_source;
                let object_changed = db_mod.3.as_deref() != Some(&object_id);

                if path_changed || status_changed || safety_changed || object_changed {
                    mod_repo::update_mod_sync_row(
                        &mut *tx,
                        mod_repo::SyncModRowUpdate {
                            new_id: &id,
                            folder_path: &stored_folder_path,
                            mods_path,
                            actual_name: &item.display_name,
                            status: current_status,
                            is_safe: next_is_safe,
                            corridor_source: next_corridor_source,
                            disabled_reason: if item.is_disabled {
                                Some(DISABLED_REASON_USER)
                            } else {
                                None
                            },
                            object_id: &object_id,
                            object_type: obj_type,
                            old_folder_path: &db_mod.1,
                            game_id,
                        },
                    )
                    .await?;

                    updated_mods_count += 1;
                }
                (id, object_id)
            } else {
                let id = generate_stable_id(game_id, &actual_folder_path);
                let (is_safe, corridor_source) =
                    crate::services::scanner::sync::helpers::classify_corridor(
                        &item.display_name,
                        safe_mode_keywords,
                    );
                mod_repo::insert_mod_with_reason_tx(
                    &mut *tx,
                    &id,
                    game_id,
                    &object_id,
                    &item.display_name,
                    &stored_folder_path,
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
                .await?;
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
                && crate::common::classifier::classify_folder(actual_path).0
                    == crate::common::classifier::NodeType::VariantContainer;

            if is_pure_object_container {
                // It's a pure container. We ensured the object exists, but we MUST DELETE
                // the mod_id that was just created for it!
                crate::repo::mod_repo::delete_mod_tx(&mut *tx, &mod_id).await?;
            }
        }
        let (is_safe, _) = crate::services::scanner::sync::helpers::classify_corridor(
            &item.display_name,
            safe_mode_keywords,
        );
        if !is_safe {
            let update = crate::services::mods::info_json::ModInfoUpdate {
                is_safe: Some(false),
                ..Default::default()
            };
            let path = std::path::Path::new(&actual_folder_path);
            let _ = crate::services::mods::info_json::update_info_json(path, &update);
        }
    }

    Ok((new_mods_count, updated_mods_count))
}
