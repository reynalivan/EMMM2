//! Read-only previews: collection contents and the apply diff.

use super::filter_target_mods_for_safe_mode;
use super::live_state::load_live_runtime_state;
use super::projection::{load_projected_collection_state, require_collection, require_game_match};
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::collections::domain::collection::{ApplyPreview, CollectionPreview};
use crate::modules::workspace::application::projected_state;
use crate::shared::errors::CollectionError;
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
    let runtime = collection::runtime::get(pool, &collection.game_id).await?;
    let active_id = runtime
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref());

    let tree_nodes = projected_state::build_preview_tree_from_projected_state(&projected_state);

    Ok(CollectionPreview {
        collection: collection::to_summary(&collection, active_id),
        tree_nodes,
        projected_state,
    })
}

pub async fn preview_apply(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    mods_path: Option<&str>,
    safe_mode_enabled: bool,
) -> Result<ApplyPreview, CollectionError> {
    let collection = require_collection(pool, collection_id).await?;
    require_game_match(&collection, game_id)?;
    if mods_path.is_some_and(|path| {
        let root = std::path::Path::new(path);
        !root.exists() || !root.is_dir()
    }) {
        return Err(CollectionError::RuntimeState(
            crate::shared::errors::RuntimeStateError::NoModsPath {
                game_id: game_id.to_string(),
            },
        ));
    }

    let (current_mods, current_objects) = load_live_runtime_state(pool, game_id).await?;
    let current_projected_state =
        projected_state::build_projected_state(&current_mods, &current_objects, mods_path);
    let current_tree_nodes =
        projected_state::build_preview_tree_from_projected_state(&current_projected_state);
    let target_state = load_projected_collection_state(pool, &collection, mods_path).await?;
    let target_mods = projected_state::mods_from_projected_state(&collection.id, &target_state);
    let target_objects =
        projected_state::objects_from_projected_state(&collection.id, &target_state);
    let effective_target_mods = filter_target_mods_for_safe_mode(target_mods, safe_mode_enabled);
    let effective_target_state =
        projected_state::build_projected_state(&effective_target_mods, &target_objects, mods_path);

    Ok(ApplyPreview {
        collection_name: collection.name,
        current_tree_nodes,
        target_tree_nodes: projected_state::build_preview_tree_from_projected_state(&target_state),
        effective_target_tree_nodes: projected_state::build_preview_tree_from_projected_state(
            &effective_target_state,
        ),
        current_state_name: None,
        current_state_is_unsaved: true,
        current_projected_state,
        target_projected_state: target_state,
        effective_target_projected_state: effective_target_state,
        safe_mode_enabled,
    })
}
