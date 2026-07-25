//! Projected-state loading and signature computation for a collection.

use crate::domain::collection::{
    CollectionMod, CollectionObject, CollectionRoot, ProjectedCollectionState,
};
use crate::domain::errors::CollectionError;
use crate::repo::collection_repo;
use crate::services::projected_state_service;
use sqlx::SqlitePool;

pub(super) fn build_projected_state_from_members(
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    mods_path: Option<&str>,
) -> ProjectedCollectionState {
    projected_state_service::build_projected_state(mods, objects, mods_path)
}

pub(crate) async fn load_projected_collection_state(
    pool: &SqlitePool,
    collection: &crate::domain::collection::Collection,
    mods_path: Option<&str>,
) -> Result<ProjectedCollectionState, CollectionError> {
    if mods_path.is_none() {
        if let Some(snapshot_json) = collection.snapshot_json.as_deref() {
            if let Some(snapshot) = projected_state_service::parse_snapshot_json(snapshot_json) {
                let active_root_count = snapshot.summary.active_root_count as i32;
                if collection.root_count != active_root_count
                    || collection.display_mod_count != active_root_count
                {
                    collection_repo::update_display_counts(pool, &collection.id, active_root_count)
                        .await?;
                }
                return Ok(snapshot);
            }
        }
    }

    let mods = collection_repo::get_mods(pool, &collection.id).await?;
    let objects = collection_repo::get_objects(pool, &collection.id).await?;
    let snapshot = build_projected_state_from_members(&mods, &objects, mods_path);
    if mods_path.is_some() {
        return Ok(snapshot);
    }

    let signature = projected_state_service::signature_for_projected_state(&snapshot);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&snapshot);

    collection_repo::update_snapshot(
        pool,
        &collection.id,
        snapshot_json.as_deref(),
        &signature,
        snapshot.summary.active_root_count as i32,
    )
    .await?;

    Ok(snapshot)
}

pub(crate) fn collection_members_from_projected_state(
    collection_id: &str,
    is_safe: bool,
    state: &ProjectedCollectionState,
) -> (
    Vec<CollectionMod>,
    Vec<CollectionObject>,
    Vec<CollectionRoot>,
) {
    let mods = projected_state_service::mods_from_projected_state(collection_id, state);
    let objects = projected_state_service::objects_from_projected_state(collection_id, state);
    let roots = projected_state_service::roots_from_projected_state(collection_id, is_safe, state);
    (mods, objects, roots)
}

pub fn compute_signature(mods: &[CollectionMod], objects: &[CollectionObject]) -> String {
    let projected_state = projected_state_service::build_projected_state(mods, objects, None);
    projected_state_service::signature_for_projected_state(&projected_state)
}
