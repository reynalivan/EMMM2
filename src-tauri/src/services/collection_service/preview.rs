//! Read-only previews: collection contents and the apply diff.

use super::live_state::load_live_runtime_state;
use super::projection::{load_projected_collection_state, require_collection, require_game_match};
use crate::domain::collection::{ApplyPreview, CollectionPreview};
use crate::domain::errors::CollectionError;
use crate::repo::collection_repo;
use crate::services::projected_state_service;
use sqlx::SqlitePool;

pub async fn get_collection_preview(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    mods_path: Option<&str>,
) -> Result<CollectionPreview, CollectionError> {
    let collection = require_collection(pool, collection_id).await?;
    require_game_match(&collection, game_id)?;

    let projected_state = load_projected_collection_state(pool, &collection, mods_path).await?;
    let runtime_snapshot =
        crate::services::collection_runtime_service::get_collection_runtime_state(
            pool,
            &collection.game_id,
        )
        .await
        .map_err(CollectionError::RuntimeState)?;
    let active_id = runtime_snapshot.active_collection_id.as_deref();

    let tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&projected_state);

    Ok(CollectionPreview {
        collection: collection_repo::to_summary(&collection, active_id),
        tree_nodes,
        projected_state,
    })
}

pub async fn preview_apply(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    mods_path: Option<&str>,
) -> Result<ApplyPreview, CollectionError> {
    let collection = require_collection(pool, collection_id).await?;
    require_game_match(&collection, game_id)?;
    if mods_path.is_some_and(|path| {
        let root = std::path::Path::new(path);
        !root.exists() || !root.is_dir()
    }) {
        return Err(CollectionError::RuntimeState(
            crate::domain::errors::RuntimeStateError::NoModsPath {
                game_id: game_id.to_string(),
            },
        ));
    }

    let (current_mods, current_objects) = load_live_runtime_state(pool, game_id).await?;
    let current_projected_state =
        projected_state_service::build_projected_state(&current_mods, &current_objects, mods_path);
    let current_tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&current_projected_state);
    let target_state = load_projected_collection_state(pool, &collection, mods_path).await?;

    Ok(ApplyPreview {
        collection_name: collection.name,
        current_tree_nodes,
        target_tree_nodes: projected_state_service::build_preview_tree_from_projected_state(
            &target_state,
        ),
        current_state_name: None,
        current_state_is_unsaved: true,
        current_projected_state,
        target_projected_state: target_state,
    })
}
