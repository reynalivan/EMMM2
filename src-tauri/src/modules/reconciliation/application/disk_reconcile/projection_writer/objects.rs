//! Object pass: applies every object found on disk to the `objects` table.

use crate::shared::errors::AppError;
use std::collections::HashMap;

use crate::modules::games::domain::models::ItemStatus;
use crate::modules::catalog::adapters::outbound::sqlite::object::ReconcileObjectRow as DbObjectRow;
use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{DiskObjectEntry, DiskProjection};
use crate::modules::reconciliation::application::disk_reconcile::path_updates::push_path_update;
use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePathKind;

use super::index::DbIndex;
use super::keys::{is_runtime_prefix_transition, runtime_logical_path_key};
use super::state::ProjectionWriteState;
use super::write::{IdentityTransitionState, OBJECT_STAGE_PREFIX};

/// The object row a disk folder resolved to. The mod pass cannot run without
/// this, so it travels as a parameter rather than as shared mutable state —
/// the pass order is then enforced by the signature, not by a runtime error.
pub(super) struct ObjectBinding {
    pub(super) id: String,
    pub(super) object_type: String,
}

pub(super) type ResolvedObjects = HashMap<String, ObjectBinding>;

struct SyncExistingObjectRequest<'a> {
    game_id: &'a str,
    existing: &'a DbObjectRow,
    persisted: &'a DbObjectRow,
    disk_object: &'a DiskObjectEntry,
    expected_status: ItemStatus,
    identity_was_staged: bool,
}

/// Aligns an already-known object row with what the disk now shows.
async fn sync_existing_object(
    conn: &mut sqlx::SqliteConnection,
    request: SyncExistingObjectRequest<'_>,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), AppError> {
    let SyncExistingObjectRequest {
        game_id,
        existing,
        persisted,
        disk_object,
        expected_status,
        identity_was_staged,
    } = request;
    let path_changed = persisted.folder_path != disk_object.folder_path;
    let has_leaked_stage_name = existing.name.starts_with(OBJECT_STAGE_PREFIX);
    if !identity_was_staged
        && !has_leaked_stage_name
        && !path_changed
        && persisted.status == expected_status
    {
        return Ok(());
    }

    crate::modules::catalog::adapters::outbound::sqlite::object::update_object_disk_identity_by_id(
        &mut *conn,
        &existing.id,
        &disk_object.name,
        &disk_object.folder_path,
        expected_status,
    )
    .await?;
    state.objects_changed = true;

    if !path_changed {
        return Ok(());
    }

    if !identity_was_staged
        && !is_runtime_prefix_transition(&existing.folder_path, &disk_object.folder_path)
    {
        let impact = crate::modules::collections::application::collection::handle_object_renamed_tx(
            &mut *conn,
            game_id,
            &persisted.folder_path,
            &disk_object.folder_path,
        )
        .await?;
        state.collection_reference_impact.merge(impact);
    }

    state.folders_changed = true;
    push_path_update(
        state.path_updates,
        DiskReconcilePathKind::Object,
        &persisted.folder_path,
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
    identity_transitions: &IdentityTransitionState,
    state: &mut ProjectionWriteState<'_>,
) -> Result<ResolvedObjects, AppError> {
    let mut resolved = ResolvedObjects::with_capacity(projection.objects.len());

    for disk_object in &projection.objects {
        let expected_status = ItemStatus::from_is_disabled(disk_object.is_disabled);
        let existing = index
            .object_by_filesystem_identity(
                disk_object
                    .filesystem_identity
                    .as_deref()
                    .unwrap_or_default(),
            )
            .or_else(|| index.object_by_key(&disk_object.folder_path_key))
            .or_else(|| {
                index.object_by_runtime_key(&runtime_logical_path_key(&disk_object.folder_path))
            });

        if let Some(existing_object) = existing {
            let persisted = identity_transitions
                .original_objects
                .get(&existing_object.id)
                .unwrap_or(existing_object);
            sync_existing_object(
                &mut *conn,
                SyncExistingObjectRequest {
                    game_id,
                    existing: existing_object,
                    persisted,
                    disk_object,
                    expected_status,
                    identity_was_staged: identity_transitions
                        .original_objects
                        .contains_key(&existing_object.id),
                },
                state,
            )
            .await?;
        }

        let mut new_objects_count = 0usize;
        let object_id = if let Some(existing_object) = existing {
            existing_object.id.clone()
        } else {
            crate::modules::catalog::application::objects::reconcile::ensure_object_exists(
                &mut *conn,
                crate::modules::catalog::domain::objects::EnsureObjectInput {
                    game_id,
                    folder_path: &disk_object.folder_path,
                    obj_name: &disk_object.name,
                    obj_type: "Other",
                    source: crate::modules::catalog::domain::objects::MatchSource::Disk,
                    db_thumbnail: None,
                    db_tags_json: "[]",
                    db_metadata_json: "{}",
                    db_hash_db_json: None,
                    db_custom_skins_json: None,
                },
                &mut new_objects_count,
            )
            .await?
        };
        crate::modules::catalog::adapters::outbound::sqlite::object::update_object_runtime_state_by_id(
            &mut *conn,
            &object_id,
            &disk_object.folder_path,
            expected_status,
        )
        .await?;
        crate::modules::catalog::adapters::outbound::sqlite::object::set_filesystem_identity_tx(
            &mut *conn,
            &object_id,
            disk_object.filesystem_identity.as_deref(),
        )
        .await?;
        crate::modules::collections::adapters::outbound::sqlite::rebind_object_references(
            &mut *conn,
            game_id,
            &disk_object.folder_path_key,
            &object_id,
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
            existing_by_id.map(|row| row.folder_path_key.clone()),
            existing.map(|row| {
                identity_transitions
                    .original_objects
                    .get(&row.id)
                    .unwrap_or(row)
                    .folder_path_key
                    .clone()
            }),
            Some(disk_object.folder_path_key.clone()),
        ];
        state
            .seen_object_keys
            .extend(touched_keys.into_iter().flatten());

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
