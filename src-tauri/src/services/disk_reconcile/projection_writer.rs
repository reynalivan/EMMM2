use std::collections::{HashMap, HashSet};
use std::path::Path;

use sqlx::FromRow;

use crate::database::models::ItemStatus;
use crate::domain::collection::CollectionReferenceImpact;
use crate::services::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::disk_snapshot::DiskProjection;
use crate::services::disk_reconcile::helpers::{
    generate_stable_mod_id, load_runtime_mod_metadata, normalize_runtime_name,
};
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::{DiskReconcilePathKind, DiskReconcilePathUpdate};

#[derive(Debug, Clone, FromRow)]
struct DbObjectRow {
    id: String,
    folder_path: String,
    folder_path_key: String,
    status: ItemStatus,
    object_type: String,
}

#[derive(Debug, Clone, FromRow)]
struct DbModRow {
    id: String,
    folder_path: String,
    folder_path_key: String,
    actual_name: String,
    status: ItemStatus,
    object_id: Option<String>,
    is_safe: bool,
    corridor_source: Option<String>,
    object_type: Option<String>,
}

async fn load_db_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbObjectRow>, String> {
    sqlx::query_as::<_, DbObjectRow>(
        "SELECT id, folder_path, folder_path_key, status, object_type FROM objects WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| error.to_string())
}

async fn load_db_mods(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbModRow>, String> {
    sqlx::query_as::<_, DbModRow>(
        "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, corridor_source, object_type FROM mods WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| error.to_string())
}

fn root_key(root: &str) -> String {
    crate::services::path_key::canonical_name_key(root)
}

fn root_key_for_folder_path(folder_path: &str) -> Option<String> {
    let first = Path::new(folder_path).components().next()?;
    Some(root_key(&first.as_os_str().to_string_lossy()))
}

pub(crate) struct ProjectionWriteRequest<'a> {
    pub game_id: &'a str,
    pub mods_path: &'a Path,
    pub safe_mode_keywords: &'a [String],
    pub projection: &'a DiskProjection,
    pub changed_roots: &'a [String],
    pub force_full: bool,
    pub path_updates: &'a mut Vec<DiskReconcilePathUpdate>,
    pub collection_reference_impact: &'a mut CollectionReferenceImpact,
    pub change_summary: &'a mut ChangeSummaryBuilder,
}

pub(crate) async fn reconcile_projection_in_tx(
    conn: &mut sqlx::SqliteConnection,
    request: ProjectionWriteRequest<'_>,
) -> Result<(bool, bool), String> {
    let game_id = request.game_id;
    let mods_path = request.mods_path;
    let safe_mode_keywords = request.safe_mode_keywords;
    let projection = request.projection;
    let changed_roots = request.changed_roots;
    let force_full = request.force_full;
    let path_updates = request.path_updates;
    let collection_reference_impact = request.collection_reference_impact;
    let change_summary = request.change_summary;

    let db_objects = load_db_objects(&mut *conn, game_id).await?;
    let db_mods = load_db_mods(&mut *conn, game_id).await?;
    let db_objects_by_key = db_objects
        .iter()
        .cloned()
        .map(|row| (row.folder_path_key.clone(), row))
        .collect::<HashMap<_, _>>();
    let db_objects_by_id = db_objects
        .iter()
        .cloned()
        .map(|row| (row.id.clone(), row))
        .collect::<HashMap<_, _>>();
    let db_mods_by_key = db_mods
        .iter()
        .cloned()
        .map(|row| (row.folder_path_key.clone(), row))
        .collect::<HashMap<_, _>>();
    let db_mods_by_path_lower = db_mods
        .iter()
        .cloned()
        .map(|row| (row.folder_path.to_ascii_lowercase(), row))
        .collect::<HashMap<_, _>>();
    let scope_root_keys = changed_roots
        .iter()
        .map(|root| root_key(root))
        .collect::<HashSet<_>>();
    let mods_root = mods_path.to_string_lossy().to_string();

    let mut object_ids_by_key = HashMap::new();
    let mut object_types_by_key = HashMap::new();
    let mut seen_object_keys = HashSet::new();
    let mut seen_mod_keys = HashSet::new();
    let mut deleted_object_keys = HashSet::new();
    let mut objects_changed = false;
    let mut folders_changed = false;

    for disk_object in &projection.objects {
        let expected_status = ItemStatus::from_is_disabled(disk_object.is_disabled);
        let existing = db_objects_by_key.get(&disk_object.folder_path_key).cloned();
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
                    let impact = crate::services::collection_service::handle_object_renamed_tx(
                        &mut *conn,
                        &existing_object.folder_path,
                        &disk_object.folder_path,
                    )
                    .await
                    .map_err(|error| {
                        format!("Failed to heal object rename in collections: {error}")
                    })?;
                    collection_reference_impact.merge(impact);

                    folders_changed = true;
                    push_path_update(
                        path_updates,
                        DiskReconcilePathKind::Object,
                        &existing_object.folder_path,
                        &disk_object.folder_path,
                    );
                    change_summary.record_object_renamed(&disk_object.name);
                }

                objects_changed = true;
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
        .await?;
        crate::repo::object_repo::update_object_runtime_state_by_id(
            &mut *conn,
            &object_id,
            &disk_object.folder_path,
            expected_status,
        )
        .await
        .map_err(|error| format!("Failed to sync object runtime state: {error}"))?;
        if new_objects_count > 0 {
            objects_changed = true;
            change_summary.record_object_added(&disk_object.name);
        }

        if let Some(existing_object) = db_objects_by_id.get(&object_id) {
            seen_object_keys.insert(existing_object.folder_path_key.clone());
        }

        let object_type = db_objects_by_id
            .get(&object_id)
            .map(|existing_object| existing_object.object_type.clone())
            .unwrap_or_else(|| "Other".to_string());

        object_ids_by_key.insert(disk_object.folder_path_key.clone(), object_id);
        object_types_by_key.insert(disk_object.folder_path_key.clone(), object_type);
        seen_object_keys.insert(disk_object.folder_path_key.clone());
    }

    for disk_mod in &projection.mods {
        let existing = db_mods_by_key
            .get(&disk_mod.folder_path_key)
            .or_else(|| db_mods_by_path_lower.get(&disk_mod.folder_path.to_ascii_lowercase()))
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
        let object_id = object_ids_by_key
            .get(&disk_mod.object_folder_path_key)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Disk Reconcile object mapping missing for '{}'",
                    disk_mod.folder_path
                )
            })?;
        let object_type = object_types_by_key
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
                    Some(&mods_root),
                )
                .await
                .map_err(|error| format!("Failed to update mod identity: {error}"))?;
                folders_changed = true;
                if path_changed {
                    push_path_update(
                        path_updates,
                        DiskReconcilePathKind::Mod,
                        &existing_mod.folder_path,
                        &disk_mod.folder_path,
                    );
                    change_summary.record_mod_renamed(&metadata.actual_name);
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
                folders_changed = true;
            }

            if path_changed {
                let impact = crate::services::collection_service::handle_mod_moved_or_renamed_tx(
                    &mut *conn,
                    &existing_mod.folder_path,
                    &disk_mod.folder_path,
                    Some(&object_id),
                )
                .await
                .map_err(|error| format!("Failed to heal mod rename in collections: {error}"))?;
                collection_reference_impact.merge(impact);
            }
        } else {
            crate::repo::mod_repo::insert_mod_with_reason_tx(
                &mut *conn,
                &new_id,
                game_id,
                &object_id,
                &metadata.actual_name,
                &disk_mod.folder_path,
                Some(&mods_root),
                metadata.status,
                &object_type,
                false,
                metadata.is_safe,
                metadata.corridor_source,
                if metadata.status.is_enabled() {
                    None
                } else {
                    Some(crate::services::corridor_constants::DISABLED_REASON_USER)
                },
            )
            .await
            .map_err(|error| format!("Failed to insert mod: {error}"))?;
            folders_changed = true;
            change_summary.record_mod_added(&metadata.actual_name);
        }

        seen_mod_keys.insert(disk_mod.folder_path_key.clone());
    }

    for db_object in &db_objects {
        let in_scope = force_full || scope_root_keys.contains(&db_object.folder_path_key);
        if !in_scope || seen_object_keys.contains(&db_object.folder_path_key) {
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
        deleted_object_keys.insert(db_object.folder_path_key.clone());
        objects_changed = true;
        folders_changed = true;
        change_summary.record_object_removed(&normalize_runtime_name(&db_object.folder_path));
    }

    for db_mod in &db_mods {
        let Some(db_root_key) = root_key_for_folder_path(&db_mod.folder_path) else {
            continue;
        };
        let in_scope = force_full || scope_root_keys.contains(&db_root_key);
        if !in_scope
            || seen_mod_keys.contains(&db_mod.folder_path_key)
            || deleted_object_keys.contains(&db_root_key)
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
        collection_reference_impact.merge(impact);

        crate::repo::mod_repo::delete_mod_tx(&mut *conn, &db_mod.id)
            .await
            .map_err(|error| {
                format!(
                    "Failed to delete stale mod '{}': {error}",
                    db_mod.folder_path
                )
            })?;
        folders_changed = true;
        change_summary.record_mod_removed(&db_mod.actual_name);
    }

    Ok((objects_changed, folders_changed))
}
