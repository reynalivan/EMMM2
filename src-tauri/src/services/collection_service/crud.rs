//! Collection lifecycle: list, create, delete, rename/update.

use super::live_state::{live_runtime_is_safe, load_game_mods_path, load_live_runtime_state};
use super::projection::{
    build_projected_state_from_members, collection_members_from_projected_state, compute_signature,
    load_projected_collection_state,
};
use crate::domain::collection::{
    CollectionMod, CollectionObject, CollectionSummary, CreateCollectionInput,
    CreateCollectionMode, UpdateCollectionInput,
};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::projected_state_service;
use sqlx::SqlitePool;

/// List all named collections for a game in the current corridor.
pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    _mods_path: Option<&str>,
) -> Result<Vec<CollectionSummary>, CollectionError> {
    let (runtime_mods, runtime_objects) = load_live_runtime_state(pool, game_id).await?;
    let runtime_signature = compute_signature(&runtime_mods, &runtime_objects);
    // Corridor-scoped list (excludes unsaved): an opposite-corridor collection
    // must never leak into the current corridor's list.
    let named_collections =
        collection_repo::list_for_corridor(pool, game_id, is_safe, false).await?;
    let active_id = named_collections
        .iter()
        .find(|collection| collection.signature.as_deref() == Some(runtime_signature.as_str()))
        .map(|collection| collection.id.clone());

    let mut summaries = Vec::with_capacity(named_collections.len());
    for c in named_collections {
        summaries.push(collection_repo::to_summary(&c, active_id.as_deref(), None));
    }

    Ok(summaries)
}

pub async fn create_collection(
    pool: &SqlitePool,
    input: CreateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    let id = uuid::Uuid::new_v4().to_string();
    let mods_path = load_game_mods_path(pool, &input.game_id).await?;
    let save_mode = input.save_mode.unwrap_or({
        if input.source_collection_id.is_some() {
            CreateCollectionMode::CloneSnapshot
        } else {
            CreateCollectionMode::SaveCurrentState
        }
    });
    let (persisted_mods, persisted_objects, roots, projected_state, collection_is_safe) =
        match save_mode {
            CreateCollectionMode::CloneSnapshot => {
                let Some(source_collection_id) = input.source_collection_id.as_deref() else {
                    return Err(CollectionError::Validation(
                        "Clone snapshot requires a source collection".to_string(),
                    ));
                };
                let source = collection_repo::get_by_id(pool, source_collection_id)
                    .await?
                    .ok_or_else(|| CollectionError::NotFound {
                        id: source_collection_id.to_string(),
                    })?;
                if source.game_id != input.game_id {
                    return Err(CollectionError::Validation(
                        "Snapshot source does not belong to the active game".to_string(),
                    ));
                }

                let snapshot =
                    load_projected_collection_state(pool, &source, mods_path.as_deref()).await?;
                let (mods, objects, roots) =
                    collection_members_from_projected_state(&id, source.is_safe, &snapshot);
                (mods, objects, roots, snapshot, source.is_safe)
            }
            CreateCollectionMode::SaveCurrentState => {
                if input.source_collection_id.is_some() {
                    return Err(CollectionError::Validation(
                        "Save current state cannot use a source collection".to_string(),
                    ));
                }

                let (mods, objects) = load_live_runtime_state(pool, &input.game_id).await?;
                if mods.is_empty() {
                    return Err(CollectionError::Validation(
                        "A collection must contain at least 1 active mod".to_string(),
                    ));
                }
                let collection_is_safe = live_runtime_is_safe(pool, &input.game_id).await?;

                let persisted_mods: Vec<CollectionMod> = mods
                    .iter()
                    .map(|entry| CollectionMod {
                        collection_id: id.clone(),
                        ..entry.clone()
                    })
                    .collect();
                let persisted_objects: Vec<CollectionObject> = objects
                    .iter()
                    .map(|entry| CollectionObject {
                        collection_id: id.clone(),
                        ..entry.clone()
                    })
                    .collect();
                let projected_state = build_projected_state_from_members(
                    &persisted_mods,
                    &persisted_objects,
                    mods_path.as_deref(),
                );
                let roots = projected_state_service::roots_from_projected_state(
                    &id,
                    collection_is_safe,
                    &projected_state,
                );
                (
                    persisted_mods,
                    persisted_objects,
                    roots,
                    projected_state,
                    collection_is_safe,
                )
            }
        };

    let signature = projected_state_service::signature_for_projected_state(&projected_state);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state);

    // 3. Save to DB
    collection_repo::create(
        pool,
        &id,
        &input.game_id,
        &input.name,
        collection_is_safe,
        false,
    )
    .await?;
    collection_repo::replace_all_state(
        pool,
        &id,
        &persisted_mods,
        &persisted_objects,
        &roots,
        Some(&signature),
        snapshot_json.as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await?;

    // Return summary
    let collection = collection_repo::get_by_id(pool, &id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.clone() })?;

    let active_collection_id = corridor_repo::get(pool, &input.game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?
        .and_then(|state| state.active_collection_id);

    Ok(collection_repo::to_summary(
        &collection,
        active_collection_id.as_deref(),
        None,
    ))
}

pub async fn delete_collection(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    let collection = collection_repo::get_by_id(pool, id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.to_string() })?;
    let mut tx = pool.begin().await?;
    corridor_repo::clear_collection_references_tx(&mut tx, id)
        .await
        .map_err(CollectionError::Corridor)?;
    corridor_repo::update_pointers_tx(&mut tx, &collection.game_id, collection.is_safe, None, None)
        .await
        .map_err(CollectionError::Corridor)?;
    collection_repo::delete_tx(&mut tx, id).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    if let Some(ref name) = input.name {
        collection_repo::rename(pool, &input.id, name).await?;
    }
    let collection = collection_repo::get_by_id(pool, &input.id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: input.id.clone(),
        })?;
    let _ = load_projected_collection_state(pool, &collection, None).await?;
    let collection = collection_repo::get_by_id(pool, &input.id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: input.id.clone(),
        })?;

    Ok(collection_repo::to_summary(&collection, None, None))
}
