//! Collection lifecycle: list, create, delete, rename/update.

use super::live_state::{live_runtime_is_safe, load_game_mods_path, load_live_runtime_state};
use super::projection::{
    collection_members_from_projected_state, compute_signature, load_projected_collection_state,
    persist_projected_state, require_collection, require_game_match,
};
use crate::domain::collection::{
    Collection, CollectionMod, CollectionObject, CollectionSummary, CreateCollectionInput,
    CreateCollectionMode, ProjectedCollectionState, UpdateCollectionInput,
};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::projected_state_service;
use sqlx::SqlitePool;

/// List all named collections for a game in the current corridor.
pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    corridor: crate::domain::corridor::Corridor,
) -> Result<Vec<CollectionSummary>, CollectionError> {
    let is_safe = corridor.is_safe();
    let (runtime_mods, runtime_objects) = load_live_runtime_state(pool, game_id).await?;
    let runtime_signature = compute_signature(&runtime_mods, &runtime_objects);
    // Corridor-scoped list (excludes unsaved): an opposite-corridor collection
    // must never leak into the current corridor's list.
    let named_collections =
        collection_repo::list_named_for_corridor(pool, game_id, is_safe).await?;
    let active_id = named_collections
        .iter()
        .find(|collection| collection.signature.as_deref() == Some(runtime_signature.as_str()))
        .map(|collection| collection.id.clone());

    Ok(named_collections
        .iter()
        .map(|collection| collection_repo::to_summary(collection, active_id.as_deref()))
        .collect())
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
    let (persisted_mods, persisted_objects, projected_state, collection_is_safe) = match save_mode {
        CreateCollectionMode::CloneSnapshot => {
            let Some(source_collection_id) = input.source_collection_id.as_deref() else {
                return Err(CollectionError::Validation(
                    "Clone snapshot requires a source collection".to_string(),
                ));
            };
            let source = require_collection(pool, source_collection_id).await?;
            if source.game_id != input.game_id {
                return Err(CollectionError::Validation(
                    "Snapshot source does not belong to the active game".to_string(),
                ));
            }

            let snapshot =
                load_projected_collection_state(pool, &source, mods_path.as_deref()).await?;
            let (mods, objects, _roots) =
                collection_members_from_projected_state(&id, source.is_safe, &snapshot);
            (mods, objects, snapshot, source.is_safe)
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
            let projected_state = projected_state_service::build_projected_state(
                &persisted_mods,
                &persisted_objects,
                mods_path.as_deref(),
            );
            (
                persisted_mods,
                persisted_objects,
                projected_state,
                collection_is_safe,
            )
        }
    };

    let mut tx = pool.begin().await?;
    collection_repo::create_tx(
        &mut tx,
        collection_repo::CreateCollectionRow {
            id: &id,
            game_id: &input.game_id,
            name: &input.name,
            is_safe: collection_is_safe,
            is_unsaved: false,
        },
    )
    .await?;
    persist_projected_state(
        &mut *tx,
        &id,
        collection_is_safe,
        &persisted_mods,
        &persisted_objects,
        &projected_state,
    )
    .await?;
    tx.commit().await?;

    let collection = require_collection(pool, &id).await?;

    let active_collection_id = corridor_repo::get(pool, &input.game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?
        .and_then(|state| state.active_collection_id);

    Ok(collection_repo::to_summary(
        &collection,
        active_collection_id.as_deref(),
    ))
}

pub async fn delete_collection(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    let collection = require_collection(pool, id).await?;
    if collection.is_unsaved {
        return Err(CollectionError::Validation(
            "Cannot delete the runtime unsaved snapshot directly".to_string(),
        ));
    }
    let projected_state = active_runtime_snapshot(pool, &collection).await?;

    let mut tx = pool.begin().await?;
    if let Some(projected_state) = projected_state.as_ref() {
        persist_unsaved_runtime_tx(&mut tx, &collection, projected_state).await?;
    }
    corridor_repo::clear_collection_references_tx(&mut tx, id)
        .await
        .map_err(CollectionError::Corridor)?;
    collection_repo::delete_tx(&mut tx, id).await?;
    tx.commit().await?;

    Ok(())
}

async fn active_runtime_snapshot(
    pool: &SqlitePool,
    collection: &Collection,
) -> Result<Option<ProjectedCollectionState>, CollectionError> {
    let (runtime_mods, runtime_objects) =
        load_live_runtime_state(pool, &collection.game_id).await?;
    let runtime_signature = compute_signature(&runtime_mods, &runtime_objects);
    if !collection_is_active(pool, collection, &runtime_signature).await? {
        return Ok(None);
    }

    let mods_path = load_game_mods_path(pool, &collection.game_id).await?;
    Ok(Some(projected_state_service::build_projected_state(
        &runtime_mods,
        &runtime_objects,
        mods_path.as_deref(),
    )))
}

async fn collection_is_active(
    pool: &SqlitePool,
    collection: &Collection,
    runtime_signature: &str,
) -> Result<bool, CollectionError> {
    let corridor = corridor_repo::get(pool, &collection.game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?;
    if corridor
        .and_then(|state| state.active_collection_id)
        .as_deref()
        == Some(collection.id.as_str())
    {
        return Ok(true);
    }

    let named =
        collection_repo::list_named_for_corridor(pool, &collection.game_id, collection.is_safe)
            .await?;
    Ok(named
        .iter()
        .find(|candidate| candidate.signature.as_deref() == Some(runtime_signature))
        .is_some_and(|candidate| candidate.id == collection.id))
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    let collection = require_collection(pool, &input.id).await?;
    require_game_match(&collection, &input.game_id)?;
    if let Some(ref name) = input.name {
        collection_repo::rename(pool, &collection, name).await?;
    }
    let collection = require_collection(pool, &input.id).await?;
    // A cache-only load repairs legacy display counts before the renamed summary is returned.
    load_projected_collection_state(pool, &collection, None).await?;
    let collection = require_collection(pool, &input.id).await?;

    Ok(collection_repo::to_summary(&collection, None))
}

async fn persist_unsaved_runtime_tx(
    conn: &mut sqlx::SqliteConnection,
    deleted_collection: &Collection,
    projected_state: &ProjectedCollectionState,
) -> Result<(), CollectionError> {
    let unsaved_id = ensure_unsaved_row_tx(&mut *conn, deleted_collection).await?;
    let (mods, objects, _) = collection_members_from_projected_state(
        &unsaved_id,
        deleted_collection.is_safe,
        projected_state,
    );
    persist_projected_state(
        &mut *conn,
        &unsaved_id,
        deleted_collection.is_safe,
        &mods,
        &objects,
        projected_state,
    )
    .await?;
    corridor_repo::update_pointers_tx(
        &mut *conn,
        &deleted_collection.game_id,
        deleted_collection.is_safe,
        Some(&unsaved_id),
    )
    .await
    .map_err(CollectionError::Corridor)
}

async fn ensure_unsaved_row_tx(
    conn: &mut sqlx::SqliteConnection,
    deleted_collection: &Collection,
) -> Result<String, CollectionError> {
    let existing_id = collection_repo::find_unsaved_id_for_corridor_tx(
        &mut *conn,
        &deleted_collection.game_id,
        deleted_collection.is_safe,
        &deleted_collection.id,
    )
    .await?;
    let should_create = existing_id.is_none();
    let unsaved_id = existing_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if !should_create {
        return Ok(unsaved_id);
    }

    let name = format!("Unsaved {}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    collection_repo::create_tx(
        &mut *conn,
        collection_repo::CreateCollectionRow {
            id: &unsaved_id,
            game_id: &deleted_collection.game_id,
            name: &name,
            is_safe: deleted_collection.is_safe,
            is_unsaved: true,
        },
    )
    .await?;
    Ok(unsaved_id)
}
