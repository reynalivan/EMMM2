//! Object pass: applies every object found on disk to the `objects` table.

use crate::domain::models::ItemStatus;
use crate::services::disk_reconcile::disk_snapshot::DiskProjection;
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::DiskReconcilePathKind;

use super::index::DbIndex;
use super::keys::{is_runtime_prefix_transition, runtime_logical_path_key};
use super::state::ProjectionWriteState;

pub(super) async fn apply_disk_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    projection: &DiskProjection,
    index: &DbIndex,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), String> {
    for disk_object in &projection.objects {
        let expected_status = ItemStatus::from_is_disabled(disk_object.is_disabled);
        let existing = index
            .objects_by_key
            .get(&disk_object.folder_path_key)
            .or_else(|| {
                index
                    .objects_by_runtime_key
                    .get(&runtime_logical_path_key(&disk_object.folder_path))
            })
            .cloned();
        if let Some(existing_object) = &existing {
            if existing_object.folder_path != disk_object.folder_path
                || existing_object.status != expected_status
            {
                crate::repo::object_repo::update_object_runtime_state_by_path(
                    &mut *conn,
                    game_id,
                    &existing_object.folder_path,
                    &disk_object.folder_path,
                    expected_status,
                )
                .await
                .map_err(|error| format!("Failed to update object runtime state: {error}"))?;

                if existing_object.folder_path != disk_object.folder_path {
                    if !is_runtime_prefix_transition(
                        &existing_object.folder_path,
                        &disk_object.folder_path,
                    ) {
                        let impact = crate::services::collection_service::handle_object_renamed_tx(
                            &mut *conn,
                            &existing_object.folder_path,
                            &disk_object.folder_path,
                        )
                        .await
                        .map_err(|error| {
                            format!("Failed to heal object rename in collections: {error}")
                        })?;
                        state.collection_reference_impact.merge(impact);
                    }

                    state.folders_changed = true;
                    push_path_update(
                        state.path_updates,
                        DiskReconcilePathKind::Object,
                        &existing_object.folder_path,
                        &disk_object.folder_path,
                    );
                    state
                        .change_summary
                        .record_object_renamed(&disk_object.name);
                }

                state.objects_changed = true;
            }
        }

        let mut new_objects_count = 0usize;
        let object_id = crate::repo::object_repo::ensure_object_exists(
            &mut *conn,
            crate::repo::object_repo::EnsureObjectInput {
                game_id,
                folder_path: &disk_object.folder_path,
                obj_name: &disk_object.name,
                obj_type: "Other",
                db_thumbnail: None,
                db_tags_json: "[]",
                db_metadata_json: "{}",
                db_hash_db_json: None,
                db_custom_skins_json: None,
            },
            &mut new_objects_count,
        )
        .await
        .map_err(|e| e.to_string())?;
        crate::repo::object_repo::update_object_runtime_state_by_id(
            &mut *conn,
            &object_id,
            &disk_object.folder_path,
            expected_status,
        )
        .await
        .map_err(|error| format!("Failed to sync object runtime state: {error}"))?;
        if new_objects_count > 0 {
            state.objects_changed = true;
            state.change_summary.record_object_added(&disk_object.name);
        }

        if let Some(existing_object) = index.objects_by_id.get(&object_id) {
            state
                .seen_object_keys
                .insert(existing_object.folder_path_key.clone());
        }
        if let Some(existing_object) = &existing {
            state
                .seen_object_keys
                .insert(existing_object.folder_path_key.clone());
        }

        let object_type = index
            .objects_by_id
            .get(&object_id)
            .map(|existing_object| existing_object.object_type.clone())
            .unwrap_or_else(|| "Other".to_string());

        state
            .object_ids_by_key
            .insert(disk_object.folder_path_key.clone(), object_id);
        state
            .object_types_by_key
            .insert(disk_object.folder_path_key.clone(), object_type);
        state
            .seen_object_keys
            .insert(disk_object.folder_path_key.clone());
    }

    Ok(())
}
