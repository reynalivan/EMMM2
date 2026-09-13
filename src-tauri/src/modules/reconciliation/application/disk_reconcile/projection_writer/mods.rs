//! Mod pass: applies every mod found on disk to the `mods` table.

use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskProjection;
use crate::modules::reconciliation::application::disk_reconcile::helpers::load_runtime_mod_metadata;
use crate::modules::reconciliation::application::disk_reconcile::path_updates::push_path_update;
use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePathKind;
use crate::modules::system::adapters::sqlite::utils::stable_ids::generate_stable_id_from_key;
use crate::shared::errors::AppError;
use crate::shared::safety_constants::{SAFETY_SOURCE_MANUAL, SAFETY_SOURCE_UNKNOWN};

use super::index::DbIndex;
use super::keys::{is_runtime_prefix_transition, runtime_logical_path_key};
use super::objects::ResolvedObjects;
use super::state::ProjectionWriteState;
use super::write::IdentityTransitionState;

pub(super) struct ModPassInput<'a> {
    pub(super) game_id: &'a str,
    pub(super) mods_root: &'a str,
    pub(super) safe_mode_keywords: &'a [String],
    pub(super) projection: &'a DiskProjection,
    pub(super) index: &'a DbIndex,
    pub(super) resolved_objects: &'a ResolvedObjects,
    pub(super) identity_transitions: &'a IdentityTransitionState,
}

pub(super) async fn apply_disk_mods(
    conn: &mut sqlx::SqliteConnection,
    input: ModPassInput<'_>,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), AppError> {
    let ModPassInput {
        game_id,
        mods_root,
        safe_mode_keywords,
        projection,
        index,
        resolved_objects,
        identity_transitions,
    } = input;

    for disk_mod in &projection.mods {
        let existing = index
            .mod_by_filesystem_identity(disk_mod.filesystem_identity.as_deref().unwrap_or_default())
            .or_else(|| index.mod_by_key(&disk_mod.folder_path_key))
            .or_else(|| index.mod_by_path_lower(&disk_mod.folder_path.to_ascii_lowercase()))
            .or_else(|| index.mod_by_runtime_key(&runtime_logical_path_key(&disk_mod.folder_path)));
        let persisted = existing.map(|row| {
            identity_transitions
                .original_mods
                .get(&row.id)
                .unwrap_or(row)
        });
        let existing_manual_safe = persisted.and_then(|row| {
            (row.safety_source.as_deref() == Some(SAFETY_SOURCE_MANUAL)).then_some(row.is_safe)
        });
        let metadata = load_runtime_mod_metadata(
            &disk_mod.absolute_path,
            &disk_mod.raw_name,
            safe_mode_keywords,
            existing_manual_safe,
        );
        let object = resolved_objects
            .get(&disk_mod.object_folder_path_key)
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "Disk Reconcile object mapping missing for '{}'",
                    disk_mod.folder_path
                ))
            })?;
        let object_id = &object.id;
        let object_type = &object.object_type;
        // The snapshot already derived this key; re-deriving it per mod would
        // walk every path component through the normalizer again.
        let new_id = generate_stable_id_from_key(game_id, &disk_mod.folder_path_key);

        if let Some(existing_mod) = existing {
            let persisted_mod = persisted.unwrap_or(existing_mod);
            let existing_safety_source = persisted_mod
                .safety_source
                .as_deref()
                .unwrap_or(SAFETY_SOURCE_UNKNOWN);
            let path_changed = persisted_mod.folder_path != disk_mod.folder_path;
            let name_changed = persisted_mod.actual_name != metadata.actual_name;
            let status_changed = persisted_mod.status != metadata.status;
            let safety_changed = persisted_mod.is_safe != metadata.is_safe
                || existing_safety_source != metadata.safety_source;
            // `mods.object_type` is a denormalized projection of its owning
            // Object. Reconcile repairs stale child rows even when the mod did
            // not move, so filters and the randomizer observe one taxonomy.
            let desired_mod_type = object_type.as_str();
            let object_changed = existing_mod.object_id.as_deref() != Some(object_id.as_str());
            let type_changed = existing_mod.object_type.as_deref() != Some(desired_mod_type);
            let id_changed = existing_mod.id != new_id;

            if path_changed || name_changed || status_changed || safety_changed || id_changed {
                if id_changed {
                    crate::modules::collections::adapters::sqlite::detach_mod_runtime_id(
                        &mut *conn,
                        game_id,
                        &existing_mod.id,
                    )
                    .await?;
                }
                crate::modules::library::adapters::sqlite::mods::update_mod_identity_tx(
                    &mut *conn,
                    &new_id,
                    &disk_mod.folder_path,
                    &metadata.actual_name,
                    metadata.status,
                    metadata.is_safe,
                    metadata.safety_source,
                    &existing_mod.id,
                    Some(mods_root),
                )
                .await?;
                state.folders_changed = true;
                if path_changed {
                    push_path_update(
                        state.path_updates,
                        DiskReconcilePathKind::Mod,
                        &persisted_mod.folder_path,
                        &disk_mod.folder_path,
                    );
                    state
                        .change_summary
                        .record_mod_renamed(&metadata.actual_name);
                }
            }

            if object_changed || type_changed {
                crate::modules::library::adapters::sqlite::mods::update_mod_object_id_and_type_tx(
                    &mut *conn,
                    &new_id,
                    object_id,
                    desired_mod_type,
                )
                .await?;
                state.folders_changed = true;
            }

            if let Some(size_bytes) = disk_mod
                .size_bytes
                .filter(|value| *value != persisted_mod.size_bytes)
            {
                crate::modules::library::adapters::sqlite::mods::update_mod_size_bytes_tx(
                    &mut *conn, &new_id, size_bytes,
                )
                .await?;
                state.folders_changed = true;
            }

            if path_changed
                && !is_runtime_prefix_transition(&persisted_mod.folder_path, &disk_mod.folder_path)
                && !identity_transitions
                    .original_mods
                    .contains_key(&existing_mod.id)
            {
                let impact = crate::modules::collections::application::collection::handle_mod_moved_or_renamed_tx(
                    &mut *conn,
                    game_id,
                    &persisted_mod.folder_path,
                    &disk_mod.folder_path,
                    Some(object_id.as_str()),
                )
                .await?;
                state.collection_reference_impact.merge(impact);
            }

            // The row may have been found under its old key; retire that one too.
            state
                .seen_mod_keys
                .insert(persisted_mod.folder_path_key.clone());
            state
                .seen_mod_keys
                .insert(existing_mod.folder_path_key.clone());
        } else {
            crate::modules::library::adapters::sqlite::mods::insert_mod_tx(
                &mut *conn,
                &new_id,
                game_id,
                object_id,
                &metadata.actual_name,
                &disk_mod.folder_path,
                Some(mods_root),
                metadata.status,
                object_type,
                false,
                metadata.is_safe,
                metadata.safety_source,
                disk_mod.size_bytes.ok_or_else(|| {
                    AppError::Internal(format!(
                        "Storage size missing for newly discovered mod '{}'",
                        disk_mod.folder_path
                    ))
                })?,
            )
            .await?;
            state.folders_changed = true;
            state.change_summary.record_mod_added(&metadata.actual_name);
        }

        crate::modules::collections::adapters::sqlite::rebind_mod_references(
            &mut *conn,
            game_id,
            &disk_mod.folder_path_key,
            &runtime_logical_path_key(&disk_mod.folder_path),
            &new_id,
            object_id,
        )
        .await?;
        crate::modules::library::adapters::sqlite::mods::set_filesystem_identity_tx(
            &mut *conn,
            &new_id,
            disk_mod.filesystem_identity.as_deref(),
        )
        .await?;

        state.seen_mod_keys.insert(disk_mod.folder_path_key.clone());
    }

    Ok(())
}
