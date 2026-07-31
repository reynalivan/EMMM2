//! Read-only previews: collection contents and the apply diff.

use super::live_state::load_live_runtime_state;
use super::projection::load_projected_collection_state;
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

    let projected_state = load_projected_collection_state(pool, &collection, mods_path).await?;
    let mods = projected_state_service::mods_from_projected_state(collection_id, &projected_state);
    let objects =
        projected_state_service::objects_from_projected_state(collection_id, &projected_state);
    let roots = projected_state_service::roots_from_projected_state(
        collection_id,
        collection.is_safe,
        &projected_state,
    );

    let corridor_snapshot = crate::services::corridor_service::get_corridor_state(
        pool,
        &collection.game_id,
        collection.is_safe,
    )
    .await
    .map_err(CollectionError::Corridor)?;
    let active_id = corridor_snapshot.active_collection_id.as_deref();

    // Build unified members list for frontend convenience
    use crate::domain::collection::CollectionMember;
    let members: Vec<CollectionMember> = mods
        .iter()
        .cloned()
        .map(CollectionMember::Mod)
        .chain(objects.iter().cloned().map(CollectionMember::Object))
        .chain(roots.iter().cloned().map(CollectionMember::Root))
        .collect();
    let tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&projected_state);

    Ok(CollectionPreview {
        collection: collection_repo::to_summary(&collection, active_id),
        members,
        mods,
        objects,
        roots,
        tree_nodes,
        projected_state,
    })
}

pub async fn preview_apply(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    is_safe: bool,
    mods_path: Option<&str>,
) -> Result<ApplyPreview, CollectionError> {
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
    if collection.is_safe != is_safe {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' belongs to the opposite corridor",
            collection_id
        )));
    }
    if mods_path.is_some_and(|path| {
        let root = std::path::Path::new(path);
        !root.exists() || !root.is_dir()
    }) {
        return Err(CollectionError::Corridor(
            crate::domain::errors::CorridorError::NoModsPath {
                game_id: game_id.to_string(),
            },
        ));
    }

    let (current_mods, current_objects) = load_live_runtime_state(pool, game_id).await?;
    let current_projected_state =
        projected_state_service::build_projected_state(&current_mods, &current_objects, mods_path);
    let current_signature =
        projected_state_service::signature_for_projected_state(&current_projected_state);
    let current_tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&current_projected_state);
    let target_state = load_projected_collection_state(pool, &collection, mods_path).await?;
    let target_mods =
        projected_state_service::mods_from_projected_state(collection_id, &target_state);
    let target_objects =
        projected_state_service::objects_from_projected_state(collection_id, &target_state);

    Ok(ApplyPreview {
        collection_name: collection.name,
        current_snapshot: Some(current_signature),
        current_mods,
        current_objects,
        current_tree_nodes,
        target_mods: target_mods.clone(),
        target_objects: target_objects.clone(),
        target_tree_nodes: projected_state_service::build_preview_tree_from_projected_state(
            &target_state,
        ),
        current_state_name: None,
        current_state_is_unsaved: true,
        current_projected_state,
        target_projected_state: target_state,
    })
}
