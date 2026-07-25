//! Prune passes: drops DB rows whose folders are gone from disk, scoped to the
//! roots this run actually covered.

use std::collections::HashSet;
use std::path::Path;

use crate::services::disk_reconcile::helpers::normalize_runtime_name;

use super::index::DbIndex;
use super::keys::root_key_for_folder_path;
use super::state::ProjectionWriteState;

pub(super) async fn prune_missing_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    index: &DbIndex,
    scope_root_keys: &HashSet<String>,
    force_full: bool,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), String> {
    for db_object in &index.objects {
        let in_scope = force_full || scope_root_keys.contains(&db_object.folder_path_key);
        if !in_scope || state.seen_object_keys.contains(&db_object.folder_path_key) {
            continue;
        }

        crate::repo::object_repo::delete_object_and_mods_by_folder(
            &mut *conn,
            game_id,
            &db_object.folder_path,
        )
        .await
        .map_err(|error| {
            format!(
                "Failed to delete object folder '{}': {error}",
                db_object.folder_path
            )
        })?;
        state
            .deleted_object_keys
            .insert(db_object.folder_path_key.clone());
        state.objects_changed = true;
        state.folders_changed = true;
        state
            .change_summary
            .record_object_removed(&normalize_runtime_name(&db_object.folder_path));
    }

    Ok(())
}

pub(super) async fn prune_missing_mods(
    conn: &mut sqlx::SqliteConnection,
    mods_path: &Path,
    index: &DbIndex,
    scope_root_keys: &HashSet<String>,
    force_full: bool,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), String> {
    for db_mod in &index.mods {
        let Some(db_root_key) = root_key_for_folder_path(&db_mod.folder_path) else {
            continue;
        };
        let in_scope = force_full || scope_root_keys.contains(&db_root_key);
        if !in_scope
            || state.seen_mod_keys.contains(&db_mod.folder_path_key)
            || state.deleted_object_keys.contains(&db_root_key)
        {
            continue;
        }

        if mods_path.join(&db_mod.folder_path).exists() {
            continue;
        }

        let impact = crate::services::collection_service::handle_mod_missing_tx(
            &mut *conn,
            &db_mod.folder_path,
        )
        .await
        .map_err(|error| {
            format!(
                "Failed to report missing collection references for '{}': {error}",
                db_mod.folder_path
            )
        })?;
        state.collection_reference_impact.merge(impact);

        crate::repo::mod_repo::delete_mod_tx(&mut *conn, &db_mod.id)
            .await
            .map_err(|error| {
                format!(
                    "Failed to delete stale mod '{}': {error}",
                    db_mod.folder_path
                )
            })?;
        state.folders_changed = true;
        state.change_summary.record_mod_removed(&db_mod.actual_name);
    }

    Ok(())
}
