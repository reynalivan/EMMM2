//! Mod pass: applies every mod found on disk to the `mods` table.

use crate::common::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::disk_reconcile::disk_snapshot::DiskProjection;
use crate::services::disk_reconcile::helpers::{generate_stable_mod_id, load_runtime_mod_metadata};
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::DiskReconcilePathKind;

use super::index::DbIndex;
use super::keys::{is_runtime_prefix_transition, runtime_logical_path_key};
use super::state::ProjectionWriteState;

pub(super) async fn apply_disk_mods(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_root: &str,
    safe_mode_keywords: &[String],
    projection: &DiskProjection,
    index: &DbIndex,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), String> {
    for disk_mod in &projection.mods {
        let existing = index
            .mods_by_key
            .get(&disk_mod.folder_path_key)
            .or_else(|| {
                index
                    .mods_by_path_lower
                    .get(&disk_mod.folder_path.to_ascii_lowercase())
            })
            .or_else(|| {
                index
                    .mods_by_runtime_key
                    .get(&runtime_logical_path_key(&disk_mod.folder_path))
            })
            .cloned();
        let existing_manual_safe = existing.as_ref().and_then(|row| {
            (row.corridor_source.as_deref() == Some(CORRIDOR_SOURCE_MANUAL)).then_some(row.is_safe)
        });
        let metadata = load_runtime_mod_metadata(
            &disk_mod.absolute_path,
            &disk_mod.raw_name,
            disk_mod.object_disabled,
            safe_mode_keywords,
            existing_manual_safe,
        );
        let object_id = state
            .object_ids_by_key
            .get(&disk_mod.object_folder_path_key)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Disk Reconcile object mapping missing for '{}'",
                    disk_mod.folder_path
                )
            })?;
        let object_type = state
            .object_types_by_key
            .get(&disk_mod.object_folder_path_key)
            .cloned()
            .unwrap_or_else(|| "Other".to_string());
        let new_id = generate_stable_mod_id(game_id, &disk_mod.folder_path);

        if let Some(existing_mod) = &existing {
            let existing_corridor_source = existing_mod
                .corridor_source
                .as_deref()
                .unwrap_or(CORRIDOR_SOURCE_UNKNOWN);
            let path_changed = existing_mod.folder_path != disk_mod.folder_path;
            let name_changed = existing_mod.actual_name != metadata.actual_name;
            let status_changed = existing_mod.status != metadata.status;
            let safety_changed = existing_mod.is_safe != metadata.is_safe
                || existing_corridor_source != metadata.corridor_source;
            let object_changed = existing_mod.object_id.as_deref() != Some(&object_id);
            let type_changed = existing_mod.object_type.as_deref() != Some(object_type.as_str());
            let id_changed = existing_mod.id != new_id;

            if path_changed || name_changed || status_changed || safety_changed || id_changed {
                crate::repo::mod_repo::update_mod_identity_tx(
                    &mut *conn,
                    &new_id,
                    &disk_mod.folder_path,
                    &metadata.actual_name,
                    metadata.status,
                    metadata.is_safe,
                    metadata.corridor_source,
                    &existing_mod.id,
                    Some(mods_root),
                )
                .await
                .map_err(|error| format!("Failed to update mod identity: {error}"))?;
                state.folders_changed = true;
                if path_changed {
                    push_path_update(
                        state.path_updates,
                        DiskReconcilePathKind::Mod,
                        &existing_mod.folder_path,
                        &disk_mod.folder_path,
                    );
                    state
                        .change_summary
                        .record_mod_renamed(&metadata.actual_name);
                }
            }

            if object_changed || type_changed {
                crate::repo::mod_repo::update_mod_object_id_and_type_tx(
                    &mut *conn,
                    &new_id,
                    &object_id,
                    &object_type,
                )
                .await
                .map_err(|error| format!("Failed to update mod object mapping: {error}"))?;
                state.folders_changed = true;
            }

            if path_changed
                && !is_runtime_prefix_transition(&existing_mod.folder_path, &disk_mod.folder_path)
            {
                let impact = crate::services::collection_service::handle_mod_moved_or_renamed_tx(
                    &mut *conn,
                    &existing_mod.folder_path,
                    &disk_mod.folder_path,
                    Some(&object_id),
                )
                .await
                .map_err(|error| format!("Failed to heal mod rename in collections: {error}"))?;
                state.collection_reference_impact.merge(impact);
            }
        } else {
            crate::repo::mod_repo::insert_mod_with_reason_tx(
                &mut *conn,
                &new_id,
                game_id,
                &object_id,
                &metadata.actual_name,
                &disk_mod.folder_path,
                Some(mods_root),
                metadata.status,
                &object_type,
                false,
                metadata.is_safe,
                metadata.corridor_source,
                if metadata.status.is_enabled() {
                    None
                } else {
                    Some(crate::common::corridor_constants::DISABLED_REASON_USER)
                },
            )
            .await
            .map_err(|error| format!("Failed to insert mod: {error}"))?;
            state.folders_changed = true;
            state.change_summary.record_mod_added(&metadata.actual_name);
        }

        if let Some(existing_mod) = &existing {
            state
                .seen_mod_keys
                .insert(existing_mod.folder_path_key.clone());
        }
        state.seen_mod_keys.insert(disk_mod.folder_path_key.clone());
    }

    Ok(())
}
