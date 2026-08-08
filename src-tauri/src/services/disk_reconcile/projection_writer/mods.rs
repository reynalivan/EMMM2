//! Mod pass: applies every mod found on disk to the `mods` table.

use crate::common::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};
use crate::domain::errors::AppError;
use crate::repo::stable_ids::generate_stable_id_from_key;
use crate::services::disk_reconcile::disk_snapshot::DiskProjection;
use crate::services::disk_reconcile::helpers::load_runtime_mod_metadata;
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::DiskReconcilePathKind;

use super::index::DbIndex;
use super::keys::{is_runtime_prefix_transition, runtime_logical_path_key};
use super::objects::ResolvedObjects;
use super::state::ProjectionWriteState;

pub(super) struct ModPassInput<'a> {
    pub(super) game_id: &'a str,
    pub(super) mods_root: &'a str,
    pub(super) safe_mode_keywords: &'a [String],
    pub(super) projection: &'a DiskProjection,
    pub(super) index: &'a DbIndex,
    pub(super) resolved_objects: &'a ResolvedObjects,
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
    } = input;

    for disk_mod in &projection.mods {
        let existing = index
            .mod_by_key(&disk_mod.folder_path_key)
            .or_else(|| index.mod_by_path_lower(&disk_mod.folder_path.to_ascii_lowercase()))
            .or_else(|| index.mod_by_runtime_key(&runtime_logical_path_key(&disk_mod.folder_path)));
        let existing_manual_safe = existing.and_then(|row| {
            (row.corridor_source.as_deref() == Some(CORRIDOR_SOURCE_MANUAL)).then_some(row.is_safe)
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
            let existing_corridor_source = existing_mod
                .corridor_source
                .as_deref()
                .unwrap_or(CORRIDOR_SOURCE_UNKNOWN);
            let path_changed = existing_mod.folder_path != disk_mod.folder_path;
            let name_changed = existing_mod.actual_name != metadata.actual_name;
            let status_changed = existing_mod.status != metadata.status;
            let safety_changed = existing_mod.is_safe != metadata.is_safe
                || existing_corridor_source != metadata.corridor_source;
            let object_changed = existing_mod.object_id.as_deref() != Some(object_id.as_str());
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
                .await?;
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
                    object_id,
                    object_type,
                )
                .await?;
                state.folders_changed = true;
            }

            if path_changed
                && !is_runtime_prefix_transition(&existing_mod.folder_path, &disk_mod.folder_path)
            {
                let impact = crate::services::collection_service::handle_mod_moved_or_renamed_tx(
                    &mut *conn,
                    &existing_mod.folder_path,
                    &disk_mod.folder_path,
                    Some(object_id.as_str()),
                )
                .await?;
                state.collection_reference_impact.merge(impact);
            }

            // The row may have been found under its old key; retire that one too.
            state
                .seen_mod_keys
                .insert(existing_mod.folder_path_key.clone());
        } else {
            crate::repo::mod_repo::insert_mod_with_reason_tx(
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
                metadata.corridor_source,
                if metadata.status.is_enabled() {
                    None
                } else {
                    Some(crate::common::corridor_constants::DISABLED_REASON_USER)
                },
            )
            .await?;
            state.folders_changed = true;
            state.change_summary.record_mod_added(&metadata.actual_name);
        }

        state.seen_mod_keys.insert(disk_mod.folder_path_key.clone());
    }

    Ok(())
}
