use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::games::domain::models::ItemStatus;
use crate::modules::reconciliation::application::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::modules::reconciliation::application::disk_reconcile::helpers::load_runtime_mod_metadata;
use crate::modules::reconciliation::application::disk_reconcile::path_updates::push_path_update;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcilePathKind, DiskReconcilePathUpdate,
};
use crate::modules::reconciliation::application::disk_reconcile::watcher_batch::{
    collect_rename_hints, WatcherRenameHints,
};
use crate::modules::system::adapters::sqlite::utils::stable_ids::generate_stable_id;
use crate::modules::workspace::api::scanner::watcher::ModWatchEvent;
use crate::modules::workspace::domain::normalizer::{is_disabled_folder, normalize_display_name};
use crate::shared::errors::AppError;

async fn load_object_type(
    conn: &mut sqlx::SqliteConnection,
    object_id: &str,
) -> Result<String, AppError> {
    crate::modules::catalog::adapters::sqlite::object::get_object_type_by_id(conn, object_id)
        .await?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::Internal(format!(
                "Disk Reconcile object type missing for object '{object_id}'"
            ))
        })
}

async fn load_existing_manual_safe(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
    mods_path: &str,
) -> Result<Option<bool>, AppError> {
    Ok(
        crate::modules::library::adapters::sqlite::mods::get_manual_is_safe_by_key(
            conn,
            game_id,
            &crate::shared::path_key::folder_path_key(folder_path, Some(mods_path)),
        )
        .await?,
    )
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

fn exact_key(path: &str) -> String {
    crate::shared::path_key::exact_location_key_for_path(Path::new(path))
}

fn exact_suffix(path: &str, prefix: &str) -> Option<PathBuf> {
    let path_key = exact_key(path);
    let prefix_key = exact_key(prefix);
    if path_key == prefix_key {
        return Some(PathBuf::new());
    }
    path_key.strip_prefix(&format!("{prefix_key}/"))?;
    let path = PathBuf::from(path.replace('\\', "/"));
    let prefix = PathBuf::from(prefix.replace('\\', "/"));
    Some(
        path.components()
            .skip(prefix.components().count())
            .collect(),
    )
}

async fn apply_mod_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    request: ModRenameHintsRequest<'_>,
) -> Result<(), AppError> {
    let root_keys = request
        .hints
        .mod_renames
        .iter()
        .filter_map(|(from, _)| {
            Path::new(from).components().next().map(|root| {
                crate::shared::path_key::folder_path_key(
                    &root.as_os_str().to_string_lossy(),
                    Some(request.mods_root),
                )
            })
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rows = crate::modules::library::api::get_reconcile_mod_rows(
        &mut *conn,
        request.game_id,
        &root_keys,
        &[],
    )
    .await?;
    let mut rows_by_path = rows
        .into_iter()
        .map(|row| (exact_key(&row.folder_path), row))
        .collect::<BTreeMap<_, _>>();
    for (hint_from, hint_to) in &request.hints.mod_renames {
        let source_key = exact_key(hint_from);
        let candidates = rows_by_path
            .get(&source_key)
            .into_iter()
            .chain(
                rows_by_path
                    .range(format!("{source_key}/")..format!("{source_key}0"))
                    .map(|(_, row)| row),
            )
            .cloned()
            .collect::<Vec<_>>();
        for mut row in candidates {
            let old_relative = row.folder_path.clone();
            let Some(suffix) = exact_suffix(&old_relative, hint_from) else {
                continue;
            };
            let new_relative = if suffix.as_os_str().is_empty() {
                hint_to.clone()
            } else {
                Path::new(hint_to)
                    .join(suffix)
                    .to_string_lossy()
                    .to_string()
            };
            let target_identity = crate::platform::fs::file_utils::filesystem_identity(
                &request.mods_path.join(&new_relative),
            );
            if row.filesystem_identity.is_none() || row.filesystem_identity != target_identity {
                continue;
            }
            let old_id = row.id.clone();

            let components = Path::new(&new_relative).components().collect::<Vec<_>>();
            if components.len() < 2 {
                continue;
            }

            let object_folder = components[0].as_os_str().to_string_lossy().to_string();
            let mod_folder = components
                .last()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .unwrap_or_default();
            let object_name = normalize_display_name(&object_folder);
            let mut new_objects_count = 0usize;
            let object_id =
                crate::modules::catalog::application::objects::reconcile::ensure_object_exists(
                    &mut *conn,
                    crate::modules::catalog::domain::objects::EnsureObjectInput {
                        game_id: request.game_id,
                        folder_path: &object_folder,
                        obj_name: &object_name,
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
                .await?;
            let object_type = load_object_type(&mut *conn, &object_id).await?;
            let existing_manual_safe = load_existing_manual_safe(
                &mut *conn,
                request.game_id,
                &old_relative,
                request.mods_root,
            )
            .await?;
            let metadata = load_runtime_mod_metadata(
                &request.mods_path.join(&new_relative),
                &mod_folder,
                request.safe_mode_keywords,
                existing_manual_safe,
            );
            let new_id = generate_stable_id(request.game_id, &new_relative);

            crate::modules::library::adapters::sqlite::mods::defer_foreign_keys_tx(&mut *conn)
                .await?;

            crate::modules::library::adapters::sqlite::mods::update_mod_identity_tx(
                &mut *conn,
                &new_id,
                &new_relative,
                &metadata.actual_name,
                metadata.status,
                metadata.is_safe,
                metadata.safety_source,
                &old_id,
                Some(request.mods_root),
            )
            .await?;

            crate::modules::library::adapters::sqlite::mods::update_mod_object_id_and_type_tx(
                &mut *conn,
                &new_id,
                &object_id,
                &object_type,
            )
            .await?;

            let impact = crate::modules::collections::application::collection::handle_mod_moved_or_renamed_tx(
                &mut *conn,
                request.game_id,
                &old_relative,
                &new_relative,
                Some(&object_id),
            )
            .await?;
            crate::modules::collections::adapters::sqlite::update_member_mod_id_for_path(
                &mut *conn,
                request.game_id,
                &new_relative,
                &new_id,
            )
            .await?;
            request.collection_reference_impact.merge(impact);

            let parent_covers_path = request.hints.object_renames.iter().any(|(old, new)| {
                exact_suffix(&old_relative, old)
                    .zip(exact_suffix(&new_relative, new))
                    .is_some_and(|(from, to)| from == to)
            });
            if !parent_covers_path {
                push_path_update(
                    &mut *request.path_updates,
                    DiskReconcilePathKind::Mod,
                    &old_relative,
                    &new_relative,
                );
                request
                    .change_summary
                    .record_mod_renamed(&metadata.actual_name);
            }
            rows_by_path.remove(&exact_key(&old_relative));
            row.folder_path = new_relative;
            row.id = new_id;
            row.status = metadata.status;
            rows_by_path.insert(exact_key(&row.folder_path), row);
        }
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
) -> Result<(), AppError> {
    let root_keys = hints
        .object_renames
        .iter()
        .map(|(old, _)| crate::shared::path_key::folder_path_key(old, None))
        .collect::<Vec<_>>();
    let rows = crate::modules::catalog::api::get_reconcile_object_rows(
        &mut *conn,
        game_id,
        &root_keys,
        &[],
    )
    .await?;
    for (old_folder, new_folder) in &hints.object_renames {
        let Some(row) = rows
            .iter()
            .find(|row| exact_key(&row.folder_path) == exact_key(old_folder))
        else {
            continue;
        };
        let identity = crate::platform::fs::file_utils::filesystem_identity(
            &Path::new(mods_root).join(new_folder),
        );
        if row.filesystem_identity.is_none() || row.filesystem_identity != identity {
            continue;
        }
        let next_status = ItemStatus::from_is_disabled(is_disabled_folder(new_folder));
        crate::modules::catalog::adapters::sqlite::object::update_object_runtime_state_by_id(
            &mut *conn,
            &row.id,
            new_folder,
            next_status,
        )
        .await?;

        let impact =
            crate::modules::collections::application::collection::handle_object_renamed_tx(
                &mut *conn, game_id, old_folder, new_folder,
            )
            .await?;
        collection_reference_impact.merge(impact);

        push_path_update(
            path_updates,
            DiskReconcilePathKind::Object,
            old_folder,
            new_folder,
        );
        change_summary.record_object_renamed(&normalize_display_name(new_folder));
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
) -> Result<(), AppError> {
    let hints = collect_rename_hints(request.mods_path, request.watcher_events);
    if hints.mod_renames.is_empty() && hints.object_renames.is_empty() {
        return Ok(());
    }

    let mods_root = request.mods_path.to_string_lossy().to_string();
    // A parent-object rename already rewrites every child row and collection
    // reference. Apply it first, then suppress redundant child hints emitted
    // by noisy watcher backends for the same physical tree move.
    apply_object_rename_hints(
        &mut *request.conn,
        request.game_id,
        &mods_root,
        &hints,
        &mut *request.path_updates,
        &mut *request.collection_reference_impact,
        &mut *request.change_summary,
    )
    .await?;
    let mut uncovered_hints = hints.clone();
    uncovered_hints.mod_renames = hints.object_renames.clone();
    uncovered_hints.mod_renames.extend(
        hints
            .mod_renames
            .iter()
            .filter_map(|(mod_from, mod_to)| {
                for (object_from, object_to) in &hints.object_renames {
                    let Some(from_suffix) = exact_suffix(mod_from, object_from) else {
                        continue;
                    };
                    let Some(to_suffix) = exact_suffix(mod_to, object_to) else {
                        continue;
                    };
                    if !from_suffix.as_os_str().is_empty() {
                        if crate::shared::path_key::exact_location_key_for_path(&from_suffix)
                            == crate::shared::path_key::exact_location_key_for_path(&to_suffix)
                        {
                            return None;
                        }
                        return Some((
                            Path::new(object_to)
                                .join(from_suffix)
                                .to_string_lossy()
                                .to_string(),
                            mod_to.clone(),
                        ));
                    }
                }
                Some((mod_from.clone(), mod_to.clone()))
            })
            .collect::<Vec<_>>(),
    );
    apply_mod_rename_hints(
        &mut *request.conn,
        ModRenameHintsRequest {
            game_id: request.game_id,
            mods_path: request.mods_path,
            mods_root: &mods_root,
            safe_mode_keywords: request.safe_mode_keywords,
            hints: &uncovered_hints,
            path_updates: &mut *request.path_updates,
            collection_reference_impact: &mut *request.collection_reference_impact,
            change_summary: &mut *request.change_summary,
        },
    )
    .await
}

#[cfg(test)]
#[path = "tests/rename_healer_tests.rs"]
mod tests;
