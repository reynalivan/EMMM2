//! Snapshots taken from the live runtime: the synthetic "Current Runtime"
//! summary and overwriting an existing collection with current state.

use super::live_state::{live_runtime_is_safe, load_game_mods_path, load_live_runtime_state};
use super::projection::{persist_projected_state_tx, require_collection, require_game_match};
use crate::domain::collection::{CollectionMod, CollectionObject, CollectionSummary};
use crate::domain::errors::CollectionError;
use crate::repo::{collection, collection::runtime};
use crate::services::projected_state;
use sqlx::SqlitePool;

pub async fn replace_collection_with_current_state(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
) -> Result<CollectionSummary, CollectionError> {
    let collection = require_collection(pool, collection_id).await?;

    require_game_match(&collection, game_id)?;
    if collection.is_draft {
        return Err(CollectionError::Validation(
            "Cannot replace an unsaved collection snapshot".to_string(),
        ));
    }

    let mods_path = load_game_mods_path(pool, game_id).await?;
    let (mods, objects) = load_live_runtime_state(pool, game_id).await?;
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
    let projected_state = projected_state::build_projected_state(
        &persisted_mods,
        &persisted_objects,
        mods_path.as_deref(),
    );
    let collection_is_safe = live_runtime_is_safe(pool, game_id).await?;

    let mut tx = pool.begin().await?;
    let draft_id = collection::runtime::get_tx(&mut tx, game_id)
        .await?
        .and_then(|runtime| runtime.draft_collection_id);
    persist_projected_state_tx(
        &mut tx,
        &collection.id,
        &persisted_mods,
        &persisted_objects,
        &projected_state,
    )
    .await?;
    collection::update_safety_summary_tx(&mut tx, &collection.id, collection_is_safe).await?;
    collection::runtime::set_active_tx(&mut tx, game_id, Some(&collection.id)).await?;
    collection::runtime::clear_draft_tx(&mut tx, game_id).await?;
    if let Some(draft_id) = draft_id {
        collection::delete_tx(&mut tx, &draft_id).await?;
    }
    tx.commit().await?;

    let updated = require_collection(pool, &collection.id).await?;
    let runtime = collection::runtime::get(pool, game_id).await?;
    let active_id = runtime
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref());

    Ok(collection::to_summary(&updated, active_id))
}
