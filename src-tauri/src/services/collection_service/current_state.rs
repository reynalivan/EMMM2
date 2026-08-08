//! Snapshots taken from the live runtime: the synthetic "Current Runtime"
//! summary and overwriting an existing collection with current state.

use super::live_state::{
    live_runtime_is_safe, load_game_mods_path, load_live_corridor_state, load_live_runtime_state,
};
use super::projection::{persist_projected_state, require_collection, require_game_match};
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
    let projected_state = projected_state_service::build_projected_state(&mods, &objects, None);
    let signature = projected_state_service::signature_for_projected_state(&projected_state);

    Ok(CollectionSummary {
        id: "__current_runtime__".to_string(),
        name: "Current Runtime".to_string(),
        is_safe: live_runtime_is_safe(pool, game_id).await?,
        is_unsaved: true,
        is_active: false,
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
    let collection = require_collection(pool, collection_id).await?;

    require_game_match(&collection, game_id)?;
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
    let projected_state = projected_state_service::build_projected_state(
        &persisted_mods,
        &persisted_objects,
        mods_path.as_deref(),
    );
    persist_projected_state(
        pool,
        &collection.id,
        collection.is_safe,
        &persisted_mods,
        &persisted_objects,
        &projected_state,
    )
    .await?;

    let updated = require_collection(pool, &collection.id).await?;
    let corridor = corridor_repo::get(pool, game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?;
    let active_id = corridor
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref());

    Ok(collection_repo::to_summary(&updated, active_id))
}
