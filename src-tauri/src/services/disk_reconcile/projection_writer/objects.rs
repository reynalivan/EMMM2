//! Object pass: applies every object found on disk to the `objects` table.

use crate::domain::errors::AppError;
use std::collections::HashMap;

use crate::domain::models::ItemStatus;
use crate::repo::object_repo::ReconcileObjectRow as DbObjectRow;
use crate::services::disk_reconcile::disk_snapshot::{DiskObjectEntry, DiskProjection};
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::DiskReconcilePathKind;

use super::index::DbIndex;
use super::keys::{is_runtime_prefix_transition, runtime_logical_path_key};
use super::state::ProjectionWriteState;

/// The object row a disk folder resolved to. The mod pass cannot run without
/// this, so it travels as a parameter rather than as shared mutable state —
/// the pass order is then enforced by the signature, not by a runtime error.
pub(super) struct ObjectBinding {
    pub(super) id: String,
    pub(super) object_type: String,
}

pub(super) type ResolvedObjects = HashMap<String, ObjectBinding>;

/// Aligns an already-known object row with what the disk now shows.
async fn sync_existing_object(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    existing: &DbObjectRow,
    disk_object: &DiskObjectEntry,
    expected_status: ItemStatus,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), AppError> {
    let path_changed = existing.folder_path != disk_object.folder_path;
    if !path_changed && existing.status == expected_status {
        return Ok(());
    }

    crate::repo::object_repo::update_object_runtime_state_by_path(
        &mut *conn,
        game_id,
        &existing.folder_path,
        &disk_object.folder_path,
        expected_status,
    )
    .await?;
    state.objects_changed = true;

    if !path_changed {
        return Ok(());
    }

    if !is_runtime_prefix_transition(&existing.folder_path, &disk_object.folder_path) {
        let impact = crate::services::collection_service::handle_object_renamed_tx(
            &mut *conn,
            &existing.folder_path,
            &disk_object.folder_path,
        )
        .await?;
        state.collection_reference_impact.merge(impact);
    }

    state.folders_changed = true;
    push_path_update(
        state.path_updates,
        DiskReconcilePathKind::Object,
        &existing.folder_path,
        &disk_object.folder_path,
    );
    state
        .change_summary
        .record_object_renamed(&disk_object.name);

    Ok(())
}

pub(super) async fn apply_disk_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    projection: &DiskProjection,
    index: &DbIndex,
    state: &mut ProjectionWriteState<'_>,
) -> Result<ResolvedObjects, AppError> {
    let mut resolved = ResolvedObjects::with_capacity(projection.objects.len());

    for disk_object in &projection.objects {
        let expected_status = ItemStatus::from_is_disabled(disk_object.is_disabled);
        let existing = index
            .object_by_key(&disk_object.folder_path_key)
            .or_else(|| {
                index.object_by_runtime_key(&runtime_logical_path_key(&disk_object.folder_path))
            });

        if let Some(existing_object) = existing {
            sync_existing_object(
                &mut *conn,
                game_id,
                existing_object,
                disk_object,
                expected_status,
                state,
            )
            .await?;
        }

        let mut new_objects_count = 0usize;
        let object_id = crate::services::objects::reconcile::ensure_object_exists(
            &mut *conn,
            crate::domain::objects::EnsureObjectInput {
                game_id,
                folder_path: &disk_object.folder_path,
                obj_name: &disk_object.name,
                obj_type: "Other",
                source: crate::domain::objects::MatchSource::Disk,
                db_thumbnail: None,
                db_tags_json: "[]",
                db_metadata_json: "{}",
                db_hash_db_json: None,
                db_custom_skins_json: None,
            },
            &mut new_objects_count,
        )
        .await?;
        crate::repo::object_repo::update_object_runtime_state_by_id(
            &mut *conn,
            &object_id,
            &disk_object.folder_path,
            expected_status,
        )
        .await?;
        if new_objects_count > 0 {
            state.objects_changed = true;
            state.change_summary.record_object_added(&disk_object.name);
        }

        state.touched_object_ids.insert(object_id.clone());

        let existing_by_id = index.object_by_id(&object_id);

        // The row may be indexed under a key the disk entry no longer produces;
        // retire every key that resolved to it so prune leaves it alone.
        let touched_keys = [
            existing_by_id.map(|row| &row.folder_path_key),
            existing.map(|row| &row.folder_path_key),
            Some(&disk_object.folder_path_key),
        ];
        state
            .seen_object_keys
            .extend(touched_keys.into_iter().flatten().cloned());

        resolved.insert(
            disk_object.folder_path_key.clone(),
            ObjectBinding {
                id: object_id,
                object_type: existing_by_id
                    .map(|row| row.object_type.clone())
                    .unwrap_or_else(|| "Other".to_string()),
            },
        );
    }

    Ok(resolved)
}
