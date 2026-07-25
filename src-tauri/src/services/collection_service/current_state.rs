//! Snapshots taken from the live runtime: the synthetic "Current Runtime"
//! summary and overwriting an existing collection with current state.

use super::live_state::{
    live_runtime_is_safe, load_game_mods_path, load_live_corridor_state, load_live_runtime_state,
};
use super::projection::build_projected_state_from_members;
use crate::domain::collection::{CollectionMod, CollectionObject, CollectionSummary};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::projected_state_service;
use sqlx::SqlitePool;

pub async fn handle_dirty_state(
    pool: &SqlitePool,
    game_id: &str,
    _is_safe: bool,
) -> Result<CollectionSummary, CollectionError> {
    let (mods, objects) = load_live_runtime_state(pool, game_id).await?;
    let projected_state = build_projected_state_from_members(&mods, &objects, None);
    let signature = projected_state_service::signature_for_projected_state(&projected_state);

    Ok(CollectionSummary {
        id: "__current_runtime__".to_string(),
        name: "Current Runtime".to_string(),
        is_safe: live_runtime_is_safe(pool, game_id).await?,
        is_unsaved: true,
        is_active: false,
        is_undo_target: false,
        signature: Some(signature),
        updated_at: chrono::Utc::now().to_rfc3339(),
        raw_member_count: projected_state.summary.active_root_count as i32,
        mod_count: projected_state.summary.active_root_count as i32,
    })
}

pub async fn replace_collection_with_current_state(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
) -> Result<CollectionSummary, CollectionError> {
    let collection = collection_repo::get_by_id(pool, collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.to_string(),
        })?;

    if collection.game_id != game_id {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to game '{}'",
            collection_id, game_id
        )));
    }
    if collection.is_unsaved {
        return Err(CollectionError::Validation(
            "Cannot replace an unsaved collection snapshot".to_string(),
        ));
    }

    let mods_path = load_game_mods_path(pool, game_id).await?;
    let (mods, objects) = load_live_corridor_state(pool, game_id, collection.is_safe).await?;
    if mods.is_empty() {
        return Err(CollectionError::Validation(
            "A collection must contain at least 1 active mod".to_string(),
        ));
    }

    let persisted_mods: Vec<CollectionMod> = mods
        .iter()
        .map(|entry| CollectionMod {
            collection_id: collection.id.clone(),
            ..entry.clone()
        })
        .collect();
    let persisted_objects: Vec<CollectionObject> = objects
        .iter()
        .map(|entry| CollectionObject {
            collection_id: collection.id.clone(),
            ..entry.clone()
        })
        .collect();
    let projected_state = build_projected_state_from_members(
        &persisted_mods,
        &persisted_objects,
        mods_path.as_deref(),
    );
    let roots = projected_state_service::roots_from_projected_state(
        &collection.id,
        collection.is_safe,
        &projected_state,
    );
    let signature = projected_state_service::signature_for_projected_state(&projected_state);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state);

    collection_repo::replace_all_state(
        pool,
        &collection.id,
        &persisted_mods,
        &persisted_objects,
        &roots,
        Some(&signature),
        snapshot_json.as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await?;

    let updated = collection_repo::get_by_id(pool, &collection.id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection.id.clone(),
        })?;
    let corridor = corridor_repo::get(pool, game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?;
    let active_id = corridor
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref());
    let undo_id = corridor
        .as_ref()
        .and_then(|state| state.undo_collection_id.as_deref());

    Ok(collection_repo::to_summary(&updated, active_id, undo_id))
}
