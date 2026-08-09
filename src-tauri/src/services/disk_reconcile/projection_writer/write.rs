//! Entry point: loads the DB index, then runs the object / mod / prune passes
//! inside the caller's transaction.

use crate::domain::errors::AppError;
use std::collections::HashSet;
use std::path::Path;

use crate::domain::collection::CollectionReferenceImpact;
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::disk_snapshot::DiskProjection;
use crate::services::disk_reconcile::types::DiskReconcilePathUpdate;

use super::index::DbIndex;
use super::keys::root_key;
use super::mods::{apply_disk_mods, ModPassInput};
use super::objects::apply_disk_objects;
use super::prune::{prune_missing_mods, prune_missing_objects};
use super::state::ProjectionWriteState;

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

    let index = DbIndex::load(&mut *conn, game_id).await?;
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

    let resolved_objects =
        apply_disk_objects(&mut *conn, game_id, projection, &index, &mut state).await?;
    apply_disk_mods(
        &mut *conn,
        ModPassInput {
            game_id,
            mods_root: &mods_root,
            safe_mode_keywords,
            projection,
            index: &index,
            resolved_objects: &resolved_objects,
        },
        &mut state,
    )
    .await?;
    prune_missing_objects(
        &mut *conn,
        game_id,
        &index,
        &scope_root_keys,
        force_full,
        &mut state,
    )
    .await?;
    prune_missing_mods(
        &mut *conn,
        mods_path,
        &index,
        &scope_root_keys,
        force_full,
        &mut state,
    )
    .await?;

    Ok(ProjectionWriteOutcome {
        objects_changed: state.objects_changed,
        folders_changed: state.folders_changed,
        touched_object_ids: state.touched_object_ids,
    })
}
