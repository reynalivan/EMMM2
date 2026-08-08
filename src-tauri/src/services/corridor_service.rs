use sqlx::SqlitePool;

use crate::domain::corridor::CorridorSnapshot;
use crate::domain::errors::CorridorError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::projected_state_service;

// ---------------------------------------------------------------------------
// corridor_service — Business logic for corridor mode switching
// ---------------------------------------------------------------------------

/// Get the current corridor state as a frontend-ready snapshot.
pub async fn get_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    corridor: crate::domain::corridor::Corridor,
) -> Result<CorridorSnapshot, CorridorError> {
    let is_safe = corridor.is_safe();
    corridor_repo::ensure_exists(pool, game_id, is_safe).await?;
    let (current_mods, current_objects) =
        crate::services::collection_service::load_live_runtime_state(pool, game_id)
            .await
            .map_err(CorridorError::from)?;
    let projected_state =
        projected_state_service::build_projected_state(&current_mods, &current_objects, None);
    let current_tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&projected_state);
    let current_signature =
        projected_state_service::signature_for_projected_state(&projected_state);

    let collections = collection_repo::list_for_game(pool, game_id)
        .await
        .map_err(CorridorError::from)?;
    let matched_collection = collections.iter().find(|collection| {
        !collection.is_unsaved
            && collection.signature.as_deref() == Some(current_signature.as_str())
    });

    let active_collection_id = matched_collection.map(|collection| collection.id.clone());
    let active_collection_name = matched_collection.map(|collection| collection.name.clone());
    let is_dirty = matched_collection.is_none();

    Ok(CorridorSnapshot {
        game_id: game_id.to_string(),
        is_safe,
        active_collection_id,
        active_collection_name,
        current_signature,
        is_dirty,
        current_mods,
        current_objects,
        current_tree_nodes,
        projected_state,
    })
}

pub(crate) async fn resolve_restore_collection(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Option<(crate::domain::collection::Collection, String)>, CorridorError> {
    let corridor = corridor_repo::get(pool, game_id, is_safe).await?;

    if let Some(active_id) = corridor
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
    {
        if let Some(collection) = collection_repo::get_by_id(pool, active_id).await? {
            if collection.game_id == game_id && collection.is_safe == is_safe {
                return Ok(Some((collection, "active_collection".to_string())));
            }

            log::warn!(
                "corridor_service: active collection pointer '{}' points outside game '{}' safe={}",
                active_id,
                game_id,
                is_safe
            );
        }

        log::warn!(
            "corridor_service: stale active collection pointer '{}' for game '{}' safe={}",
            active_id,
            game_id,
            is_safe
        );
    }

    if let Some(collection) =
        collection_repo::find_unsaved_for_corridor(pool, game_id, is_safe, None).await?
    {
        return Ok(Some((collection, "unsaved".to_string())));
    }

    Ok(None)
}

#[cfg(test)]
#[path = "tests/corridor_service_tests.rs"]
mod tests;
