//! Collection lifecycle: list, create, delete, rename/update.

use super::live_state::{live_runtime_is_safe, load_game_mods_path, load_live_runtime_state};
use super::projection::{
    collection_members_from_projected_state, load_projected_collection_state,
    persist_projected_state, require_collection, require_game_match,
};
use crate::modules::collections::domain::collection::{
    CollectionMod, CollectionObject, CollectionSummary, CreateCollectionInput,
    CreateCollectionMode, UpdateCollectionInput,
};
use crate::shared::errors::CollectionError;
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::workspace::application::projected_state;
use sqlx::SqlitePool;

/// List every named collection for a game. Safety is display metadata only.
pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<CollectionSummary>, CollectionError> {
    let named_collections = collection::list_for_game(pool, game_id).await?;
    let active_id = collection::runtime::get(pool, game_id)
        .await?
        .and_then(|state| state.active_collection_id);

    let mut summaries = Vec::with_capacity(named_collections.len());
    for collection in named_collections {
        let mut current = collection.clone();
        let safety =
            collection::member_safety_summary(pool, &collection.game_id, &collection.id)
                .await?;
        current.is_safe = !safety.contains_unsafe;
        let mut summary = collection::to_summary(&current, active_id.as_deref());
        summary.is_safety_classified = safety.is_fully_classified;
        summaries.push(summary);
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
    let runtime_before_create = collection::runtime::get(pool, &input.game_id).await?;
    let draft_to_consume = match save_mode {
        CreateCollectionMode::SaveCurrentState => runtime_before_create
            .as_ref()
            .and_then(|runtime| runtime.draft_collection_id.clone()),
        CreateCollectionMode::CloneSnapshot => {
            let source_id = input.source_collection_id.as_deref();
            runtime_before_create
                .as_ref()
                .and_then(|runtime| runtime.draft_collection_id.as_deref())
                .filter(|draft_id| Some(*draft_id) == source_id)
                .map(str::to_owned)
        }
    };
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
            let (mods, objects) = collection_members_from_projected_state(&id, &snapshot);
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
            let projected_state = projected_state::build_projected_state(
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
    collection::create_tx(
        &mut tx,
        collection::CreateCollectionRow {
            id: &id,
            game_id: &input.game_id,
            name: &input.name,
            is_safe: collection_is_safe,
        },
    )
    .await?;
    persist_projected_state(
        &mut *tx,
        &id,
        &persisted_mods,
        &persisted_objects,
        &projected_state,
    )
    .await?;
    if let Some(draft_id) = draft_to_consume.as_deref() {
        super::runtime::ensure_rollback_draft_is_unreferenced(&mut tx, draft_id).await?;
    }
    if save_mode == CreateCollectionMode::SaveCurrentState {
        collection::runtime::set_active_tx(&mut tx, &input.game_id, Some(&id)).await?;
        collection::runtime::clear_draft_tx(&mut tx, &input.game_id).await?;
    } else if draft_to_consume.is_some() {
        collection::runtime::clear_draft_tx(&mut tx, &input.game_id).await?;
    }
    if let Some(draft_id) = draft_to_consume.as_deref() {
        collection::delete_tx(&mut tx, draft_id).await?;
    }
    tx.commit().await?;

    let collection = require_collection(pool, &id).await?;

    let active_collection_id = collection::runtime::get(pool, &input.game_id)
        .await?
        .and_then(|state| state.active_collection_id);

    Ok(collection::to_summary(
        &collection,
        active_collection_id.as_deref(),
    ))
}

pub async fn delete_collection(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    let collection = require_collection(pool, id).await?;
    if collection.is_draft {
        return Err(CollectionError::Validation(
            "Cannot delete the runtime unsaved snapshot directly".to_string(),
        ));
    }
    let mut tx = pool.begin().await?;
    collection::runtime::clear_collection_references_tx(&mut tx, id).await?;
    collection::delete_tx(&mut tx, id).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    let collection = require_collection(pool, &input.id).await?;
    require_game_match(&collection, &input.game_id)?;
    if let Some(ref name) = input.name {
        collection::rename(pool, &collection, name).await?;
    }
    let collection = require_collection(pool, &input.id).await?;


    Ok(collection::to_summary(&collection, None))
}
