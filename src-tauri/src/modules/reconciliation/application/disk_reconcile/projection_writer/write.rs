//! Entry point: loads the DB index, then runs the object / mod / prune passes
//! inside the caller's transaction.

use crate::shared::errors::AppError;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::modules::collections::domain::collection::{CollectionPathRewrite, CollectionReferenceImpact};
use crate::modules::reconciliation::application::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskProjection;
use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePathUpdate;

use super::index::DbIndex;
use super::keys::root_key;
use super::mods::{apply_disk_mods, ModPassInput};
use super::objects::apply_disk_objects;

pub(super) const OBJECT_STAGE_PREFIX: &str = ".emmm-reconcile-object-stage-";
use super::prune::{prune_missing_mods, prune_missing_objects, PruneScope};
use super::state::ProjectionWriteState;

#[derive(Default)]
pub(super) struct IdentityTransitionState {
    pub(super) original_objects: HashMap<String, crate::modules::catalog::adapters::outbound::sqlite::object::ReconcileObjectRow>,
    pub(super) original_mods: HashMap<String, crate::modules::library::adapters::outbound::sqlite::mods::ReconcileModRow>,
    mod_final_ids: HashMap<String, String>,
    collection_ids_to_refresh: Vec<String>,
}

fn disk_object_identities(projection: &DiskProjection) -> HashSet<&str> {
    projection
        .objects
        .iter()
        .filter_map(|entry| entry.filesystem_identity.as_deref())
        .collect()
}

fn disk_mod_identities(projection: &DiskProjection) -> HashSet<&str> {
    projection
        .mods
        .iter()
        .filter_map(|entry| entry.filesystem_identity.as_deref())
        .collect()
}

async fn record_and_delete_displaced_object(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object: &crate::modules::catalog::adapters::outbound::sqlite::object::ReconcileObjectRow,
    index: &DbIndex,
    disk_mod_keys: &HashSet<&str>,
    state: &mut ProjectionWriteState<'_>,
) -> Result<(), AppError> {
    for child in index
        .mods
        .iter()
        .filter(|row| row.object_id.as_deref() == Some(object.id.as_str()))
    {
        if !disk_mod_keys.contains(child.folder_path_key.as_str()) {
            let impact = crate::modules::collections::application::collection::handle_mod_missing_tx(
                &mut *conn,
                game_id,
                &child.folder_path,
            )
            .await?;
            state.collection_reference_impact.merge(impact);
        }
        state.change_summary.record_mod_removed(&child.actual_name);
    }
    crate::modules::catalog::adapters::outbound::sqlite::object::delete_object_and_mods_by_folder(
        &mut *conn,
        game_id,
        &object.folder_path,
    )
    .await?;
    state
        .deleted_object_keys
        .insert(object.folder_path_key.clone());
    state.touched_object_ids.insert(object.id.clone());
    state.objects_changed = true;
    state.folders_changed = true;
    state
        .change_summary
        .record_object_removed(&crate::modules::workspace::domain::normalizer::normalize_display_name(
            &object.folder_path,
        ));
    Ok(())
}

async fn prepare_identity_transitions(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    projection: &DiskProjection,
    initial: &DbIndex,
    state: &mut ProjectionWriteState<'_>,
) -> Result<IdentityTransitionState, AppError> {
    let mut transitions = IdentityTransitionState::default();
    let object_identities = disk_object_identities(projection);
    let disk_mod_keys = projection
        .mods
        .iter()
        .map(|entry| entry.folder_path_key.as_str())
        .collect::<HashSet<_>>();

    for disk_object in &projection.objects {
        let Some(identity) = disk_object.filesystem_identity.as_deref() else {
            continue;
        };
        let Some(source) = initial.object_by_filesystem_identity(identity) else {
            continue;
        };
        if source.folder_path_key == disk_object.folder_path_key {
            continue;
        }
        if let Some(occupant) = initial.object_by_key(&disk_object.folder_path_key) {
            let occupant_is_elsewhere = occupant
                .filesystem_identity
                .as_deref()
                .is_some_and(|value| object_identities.contains(value));
            if occupant.id != source.id && !occupant_is_elsewhere {
                record_and_delete_displaced_object(
                    &mut *conn,
                    game_id,
                    occupant,
                    initial,
                    &disk_mod_keys,
                    state,
                )
                .await?;
            }
        }
        transitions
            .original_objects
            .insert(source.id.clone(), source.clone());
    }

    let object_rewrites = projection
        .objects
        .iter()
        .filter_map(|disk_object| {
            let identity = disk_object.filesystem_identity.as_deref()?;
            let source = initial.object_by_filesystem_identity(identity)?;
            (source.folder_path != disk_object.folder_path
                && !super::keys::is_runtime_prefix_transition(
                    &source.folder_path,
                    &disk_object.folder_path,
                ))
            .then(|| CollectionPathRewrite {
                from: crate::modules::collections::application::collection::logical_collection_path(
                    &source.folder_path,
                ),
                to: crate::modules::collections::application::collection::logical_collection_path(
                    &disk_object.folder_path,
                ),
            })
        })
        .collect::<Vec<_>>();

    for (position, source) in transitions.original_objects.values().enumerate() {
        let stage = format!("{OBJECT_STAGE_PREFIX}{position}-{}", uuid::Uuid::new_v4());
        crate::modules::catalog::adapters::outbound::sqlite::object::stage_object_identity_tx(&mut *conn, &source.id, &stage).await?;
    }

    let after_objects = DbIndex::load(&mut *conn, game_id).await?;
    let mod_identities = disk_mod_identities(projection);
    let mut mod_sources = Vec::new();
    for disk_mod in &projection.mods {
        let Some(identity) = disk_mod.filesystem_identity.as_deref() else {
            continue;
        };
        let Some(source) = after_objects.mod_by_filesystem_identity(identity) else {
            continue;
        };
        let final_id = crate::modules::system::adapters::outbound::sqlite::utils::stable_ids::generate_stable_id_from_key(
            game_id,
            &disk_mod.folder_path_key,
        );
        if source.id == final_id && source.folder_path_key == disk_mod.folder_path_key {
            continue;
        }
        if let Some(occupant) = after_objects
            .mod_by_id(&final_id)
            .or_else(|| after_objects.mod_by_key(&disk_mod.folder_path_key))
            .or_else(|| after_objects.mod_by_path_lower(&disk_mod.folder_path.to_ascii_lowercase()))
        {
            let occupant_is_elsewhere = occupant
                .filesystem_identity
                .as_deref()
                .is_some_and(|value| mod_identities.contains(value));
            if occupant.id != source.id && !occupant_is_elsewhere {
                crate::modules::library::adapters::outbound::sqlite::mods::delete_mod_tx(&mut *conn, &occupant.id).await?;
                state.folders_changed = true;
                state
                    .change_summary
                    .record_mod_removed(&occupant.actual_name);
            }
        }
        mod_sources.push((source.clone(), final_id));
    }

    let mod_rewrites = projection
        .mods
        .iter()
        .filter_map(|disk_mod| {
            let identity = disk_mod.filesystem_identity.as_deref()?;
            let source = after_objects.mod_by_filesystem_identity(identity)?;
            (source.folder_path != disk_mod.folder_path
                && !super::keys::is_runtime_prefix_transition(
                    &source.folder_path,
                    &disk_mod.folder_path,
                ))
            .then(|| CollectionPathRewrite {
                from: crate::modules::collections::application::collection::logical_collection_path(
                    &source.folder_path,
                ),
                to: crate::modules::collections::application::collection::logical_collection_path(
                    &disk_mod.folder_path,
                ),
            })
        })
        .collect::<Vec<_>>();
    let staged_collections =
        crate::modules::collections::application::collection::stage_identity_path_transitions_tx(
            &mut *conn,
            game_id,
            &object_rewrites,
            &mod_rewrites,
        )
        .await?;
    state
        .collection_reference_impact
        .merge(staged_collections.impact);
    transitions.collection_ids_to_refresh = staged_collections.affected_collection_ids;

    // Primary-key swaps are legal only when every source first leaves the
    // path-derived ID namespace. Foreign keys are repaired before commit.
    crate::modules::library::adapters::outbound::sqlite::mods::defer_foreign_keys_tx(&mut *conn).await?;
    for (position, (source, final_id)) in mod_sources.into_iter().enumerate() {
        crate::modules::collections::adapters::outbound::sqlite::detach_mod_runtime_id(&mut *conn, game_id, &source.id)
            .await?;
        let temp_id = format!(
            "emmm-reconcile-mod-stage-{position}-{}",
            uuid::Uuid::new_v4()
        );
        let temp_path = format!(".emmm-reconcile-mod-stage/{temp_id}");
        crate::modules::library::adapters::outbound::sqlite::mods::stage_mod_identity_tx(&mut *conn, &temp_id, &temp_path, &source.id)
            .await?;
        crate::modules::library::adapters::outbound::sqlite::mods::rewrite_dependent_mod_ids_tx(&mut *conn, &source.id, &temp_id)
            .await?;
        transitions.original_mods.insert(temp_id.clone(), source);
        transitions.mod_final_ids.insert(temp_id, final_id);
    }

    Ok(transitions)
}

async fn finalize_mod_reference_transitions(
    conn: &mut sqlx::SqliteConnection,
    transitions: &IdentityTransitionState,
) -> Result<(), AppError> {
    for (temp_id, final_id) in &transitions.mod_final_ids {
        crate::modules::library::adapters::outbound::sqlite::mods::rewrite_dependent_mod_ids_tx(&mut *conn, temp_id, final_id).await?;
    }
    Ok(())
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
    pub protected_object_keys: &'a HashSet<String>,
    pub protected_mod_keys: &'a HashSet<String>,
}

/// What the write passes touched, so the caller can refresh the runtime
/// projection for exactly those objects instead of rebuilding the whole game.
pub(crate) struct ProjectionWriteOutcome {
    pub(crate) objects_changed: bool,
    pub(crate) folders_changed: bool,
    pub(crate) touched_object_ids: HashSet<String>,
}

pub(crate) async fn reconcile_projection_in_tx(
    conn: &mut sqlx::SqliteConnection,
    request: ProjectionWriteRequest<'_>,
) -> Result<ProjectionWriteOutcome, AppError> {
    let game_id = request.game_id;
    let mods_path = request.mods_path;
    let safe_mode_keywords = request.safe_mode_keywords;
    let projection = request.projection;
    let changed_roots = request.changed_roots;
    let force_full = request.force_full;
    let protected_object_keys = request.protected_object_keys;
    let protected_mod_keys = request.protected_mod_keys;

    let initial_index = DbIndex::load(&mut *conn, game_id).await?;
    let scope_root_keys = changed_roots
        .iter()
        .map(|root| root_key(root))
        .collect::<HashSet<_>>();
    let mods_root = mods_path.to_string_lossy().to_string();

    let mut state = ProjectionWriteState {
        path_updates: request.path_updates,
        collection_reference_impact: request.collection_reference_impact,
        change_summary: request.change_summary,
        seen_object_keys: HashSet::new(),
        seen_mod_keys: HashSet::new(),
        deleted_object_keys: HashSet::new(),
        touched_object_ids: HashSet::new(),
        objects_changed: false,
        folders_changed: false,
    };

    let identity_transitions =
        prepare_identity_transitions(&mut *conn, game_id, projection, &initial_index, &mut state)
            .await?;
    let index = DbIndex::load(&mut *conn, game_id).await?;

    let resolved_objects = apply_disk_objects(
        &mut *conn,
        game_id,
        projection,
        &index,
        &identity_transitions,
        &mut state,
    )
    .await?;
    apply_disk_mods(
        &mut *conn,
        ModPassInput {
            game_id,
            mods_root: &mods_root,
            safe_mode_keywords,
            projection,
            index: &index,
            resolved_objects: &resolved_objects,
            identity_transitions: &identity_transitions,
        },
        &mut state,
    )
    .await?;
    finalize_mod_reference_transitions(&mut *conn, &identity_transitions).await?;
    let prune_scope = PruneScope {
        scope_root_keys: &scope_root_keys,
        force_full,
        protected_object_keys,
        protected_mod_keys,
    };
    prune_missing_mods(
        &mut *conn,
        game_id,
        mods_path,
        &index,
        &prune_scope,
        &mut state,
    )
    .await?;
    prune_missing_objects(&mut *conn, game_id, &index, &prune_scope, &mut state).await?;
    crate::modules::collections::application::collection::refresh_collection_signatures_tx(
        &mut *conn,
        &identity_transitions.collection_ids_to_refresh,
    )
    .await?;

    Ok(ProjectionWriteOutcome {
        objects_changed: state.objects_changed,
        folders_changed: state.folders_changed,
        touched_object_ids: state.touched_object_ids,
    })
}
