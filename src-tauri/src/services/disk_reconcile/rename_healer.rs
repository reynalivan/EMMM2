use std::path::Path;

use crate::database::models::ItemStatus;
use crate::domain::collection::CollectionReferenceImpact;
use crate::services::corridor_constants::CORRIDOR_SOURCE_MANUAL;
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::helpers::{
    generate_stable_mod_id, is_disabled_runtime_name, load_runtime_mod_metadata,
    normalize_runtime_name,
};
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::{DiskReconcilePathKind, DiskReconcilePathUpdate};
use crate::services::disk_reconcile::watcher_batch::{collect_rename_hints, WatcherRenameHints};
use crate::services::scanner::watcher::ModWatchEvent;

async fn load_object_type(
    conn: &mut sqlx::SqliteConnection,
    object_id: &str,
) -> Result<String, String> {
    sqlx::query_scalar::<_, Option<String>>("SELECT object_type FROM objects WHERE id = ?")
        .bind(object_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| error.to_string())?
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Disk Reconcile object type missing for object '{object_id}'"))
}

async fn load_existing_manual_safe(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
    mods_path: &str,
) -> Result<Option<bool>, String> {
    let row = sqlx::query_as::<_, (bool, Option<String>)>(
        "SELECT COALESCE(is_safe, 1), corridor_source FROM mods WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(game_id)
    .bind(crate::services::path_key::folder_path_key(
        folder_path,
        Some(mods_path),
    ))
    .fetch_optional(&mut *conn)
    .await
    .map_err(|error| error.to_string())?;

    Ok(row.and_then(|(is_safe, corridor_source)| {
        (corridor_source.as_deref() == Some(CORRIDOR_SOURCE_MANUAL)).then_some(is_safe)
    }))
}

struct ModRenameHintsRequest<'a> {
    game_id: &'a str,
    mods_path: &'a Path,
    mods_root: &'a str,
    safe_mode_keywords: &'a [String],
    hints: &'a WatcherRenameHints,
    path_updates: &'a mut Vec<DiskReconcilePathUpdate>,
    collection_reference_impact: &'a mut CollectionReferenceImpact,
    change_summary: &'a mut ChangeSummaryBuilder,
}

async fn apply_mod_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    request: ModRenameHintsRequest<'_>,
) -> Result<(), String> {
    for (old_relative, new_relative) in &request.hints.mod_renames {
        let mod_exists = crate::repo::mod_repo::get_mod_id_and_status_by_path(
            &mut *conn,
            old_relative,
            request.game_id,
        )
        .await
        .map_err(|error| error.to_string())?;
        let Some((old_id, _object_id, _status)) = mod_exists else {
            continue;
        };

        let components = Path::new(new_relative).components().collect::<Vec<_>>();
        if components.len() != 2 {
            continue;
        }

        let object_folder = components[0].as_os_str().to_string_lossy().to_string();
        let mod_folder = components[1].as_os_str().to_string_lossy().to_string();
        let object_name = normalize_runtime_name(&object_folder);
        let mut new_objects_count = 0usize;
        let object_id = crate::repo::object_repo::ensure_object_exists(
            &mut *conn,
            crate::repo::object_repo::EnsureObjectInput {
                game_id: request.game_id,
                folder_path: &object_folder,
                obj_name: &object_name,
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
        let object_type = load_object_type(&mut *conn, &object_id).await?;
        let existing_manual_safe =
            load_existing_manual_safe(&mut *conn, request.game_id, old_relative, request.mods_root)
                .await?;
        let metadata = load_runtime_mod_metadata(
            &request.mods_path.join(new_relative),
            &mod_folder,
            is_disabled_runtime_name(&object_folder),
            request.safe_mode_keywords,
            existing_manual_safe,
        );
        let new_id = generate_stable_mod_id(request.game_id, new_relative);

        crate::repo::mod_repo::update_mod_identity_tx(
            &mut *conn,
            &new_id,
            new_relative,
            &metadata.actual_name,
            metadata.status,
            metadata.is_safe,
            metadata.corridor_source,
            &old_id,
            Some(request.mods_root),
        )
        .await
        .map_err(|error| error.to_string())?;

        crate::repo::mod_repo::update_mod_object_id_and_type_tx(
            &mut *conn,
            &new_id,
            &object_id,
            &object_type,
        )
        .await
        .map_err(|error| error.to_string())?;

        let impact = crate::services::collection_service::handle_mod_moved_or_renamed_tx(
            &mut *conn,
            old_relative,
            new_relative,
            Some(&object_id),
        )
        .await
        .map_err(|error| format!("Failed to heal mod rename in collections: {error}"))?;
        request.collection_reference_impact.merge(impact);

        push_path_update(
            &mut *request.path_updates,
            DiskReconcilePathKind::Mod,
            old_relative,
            new_relative,
        );
        request
            .change_summary
            .record_mod_renamed(&metadata.actual_name);
    }

    Ok(())
}

async fn apply_object_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_root: &str,
    hints: &WatcherRenameHints,
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    collection_reference_impact: &mut CollectionReferenceImpact,
    change_summary: &mut ChangeSummaryBuilder,
) -> Result<(), String> {
    for (old_folder, new_folder) in &hints.object_renames {
        let next_status = ItemStatus::from_is_disabled(is_disabled_runtime_name(new_folder));
        crate::repo::object_repo::update_object_runtime_state_by_path(
            &mut *conn,
            game_id,
            old_folder,
            new_folder,
            next_status,
        )
        .await
        .map_err(|error| format!("Failed to update object folder path: {error}"))?;

        for (old_sep, new_sep) in [
            (format!("{old_folder}\\"), format!("{new_folder}\\")),
            (format!("{old_folder}/"), format!("{new_folder}/")),
        ] {
            crate::repo::mod_repo::update_child_paths_tx(
                &mut *conn,
                game_id,
                &old_sep,
                &new_sep,
                Some(mods_root),
            )
            .await
            .map_err(|error| format!("Failed to update child paths: {error}"))?;
        }

        let impact = crate::services::collection_service::handle_object_renamed_tx(
            &mut *conn, old_folder, new_folder,
        )
        .await
        .map_err(|error| format!("Failed to heal object rename in collections: {error}"))?;
        collection_reference_impact.merge(impact);

        push_path_update(
            path_updates,
            DiskReconcilePathKind::Object,
            old_folder,
            new_folder,
        );
        change_summary.record_object_renamed(&normalize_runtime_name(new_folder));
    }

    Ok(())
}

pub(crate) struct WatcherRenameHintsApplyRequest<'a> {
    pub conn: &'a mut sqlx::SqliteConnection,
    pub game_id: &'a str,
    pub mods_path: &'a Path,
    pub safe_mode_keywords: &'a [String],
    pub watcher_events: &'a [ModWatchEvent],
    pub path_updates: &'a mut Vec<DiskReconcilePathUpdate>,
    pub collection_reference_impact: &'a mut CollectionReferenceImpact,
    pub change_summary: &'a mut ChangeSummaryBuilder,
}

pub(crate) async fn apply_watcher_rename_hints(
    request: WatcherRenameHintsApplyRequest<'_>,
) -> Result<(), String> {
    let hints = collect_rename_hints(request.mods_path, request.watcher_events);
    if hints.mod_renames.is_empty() && hints.object_renames.is_empty() {
        return Ok(());
    }

    let mods_root = request.mods_path.to_string_lossy().to_string();
    apply_mod_rename_hints(
        &mut *request.conn,
        ModRenameHintsRequest {
            game_id: request.game_id,
            mods_path: request.mods_path,
            mods_root: &mods_root,
            safe_mode_keywords: request.safe_mode_keywords,
            hints: &hints,
            path_updates: &mut *request.path_updates,
            collection_reference_impact: &mut *request.collection_reference_impact,
            change_summary: &mut *request.change_summary,
        },
    )
    .await?;
    apply_object_rename_hints(
        &mut *request.conn,
        request.game_id,
        &mods_root,
        &hints,
        &mut *request.path_updates,
        &mut *request.collection_reference_impact,
        &mut *request.change_summary,
    )
    .await
}
