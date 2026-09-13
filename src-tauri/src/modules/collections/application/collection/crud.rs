//! Collection lifecycle: list, create, delete, rename/update.

use super::live_state::{
    live_runtime_is_safe, live_runtime_matches_collection_tx, load_game_mods_path,
    load_live_runtime_state,
};
use super::projection::{
    collection_members_from_projected_state, load_projected_collection_state,
    persist_projected_state, persist_projected_state_tx, require_collection, require_game_match,
};
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::collections::domain::collection::{
    CollectionMod, CollectionObject, CollectionSummary, CreateCollectionInput,
    CreateCollectionMode, UpdateCollectionInput,
};
use crate::modules::workspace::application::projected_state;
use crate::shared::errors::CollectionError;
use sqlx::SqlitePool;

/// Result of saving the live runtime as a passive collection snapshot.
///
/// `created` is false when a named collection already represents the same
/// effective runtime state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassiveCollectionSnapshot {
    pub collection_id: String,
    pub collection_name: String,
    pub created: bool,
}

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
            collection::member_safety_summary(pool, &collection.game_id, &collection.id).await?;
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
    persist_projected_state_tx(
        &mut tx,
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

/// Save the current live state without changing the active collection or draft.
///
/// Existing named collections with the same projected signature and effective
/// membership are reused. A different state with the requested canonical name
/// gets the first available ` (n)` suffix.
pub async fn snapshot_live_state_passively(
    pool: &SqlitePool,
    game_id: &str,
    requested_name: &str,
) -> Result<PassiveCollectionSnapshot, CollectionError> {
    let requested_name = requested_name.trim();
    if requested_name.is_empty() {
        return Err(CollectionError::Validation(
            "Collection name cannot be empty".to_string(),
        ));
    }

    let (mods, objects) = load_live_runtime_state(pool, game_id).await?;
    if mods.is_empty() {
        return Err(CollectionError::Validation(
            "A collection must contain at least 1 active mod".to_string(),
        ));
    }
    let is_safe = live_runtime_is_safe(pool, game_id).await?;
    let projected_state = projected_state::build_projected_state(
        &mods,
        &objects,
        load_game_mods_path(pool, game_id).await?.as_deref(),
    );
    let signature = projected_state::signature_for_projected_state(&projected_state);

    let mut tx = pool.begin().await?;
    for collection_id in collection::named_ids_by_signature_tx(&mut tx, game_id, &signature).await?
    {
        if live_runtime_matches_collection_tx(&mut tx, game_id, &collection_id).await? {
            let existing = collection::get_by_id_tx(&mut tx, &collection_id)
                .await?
                .ok_or_else(|| CollectionError::NotFound {
                    id: collection_id.clone(),
                })?;
            tx.commit().await?;
            return Ok(PassiveCollectionSnapshot {
                collection_id: existing.id,
                collection_name: existing.name,
                created: false,
            });
        }
    }

    let name = next_available_collection_name(&mut tx, game_id, requested_name).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let persisted_mods = mods
        .into_iter()
        .map(|member| CollectionMod {
            collection_id: id.clone(),
            ..member
        })
        .collect::<Vec<_>>();
    let persisted_objects = objects
        .into_iter()
        .map(|member| CollectionObject {
            collection_id: id.clone(),
            ..member
        })
        .collect::<Vec<_>>();

    collection::create_tx(
        &mut tx,
        collection::CreateCollectionRow {
            id: &id,
            game_id,
            name: &name,
            is_safe,
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
    tx.commit().await?;

    Ok(PassiveCollectionSnapshot {
        collection_id: id,
        collection_name: name,
        created: true,
    })
}

async fn next_available_collection_name(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    requested_name: &str,
) -> Result<String, CollectionError> {
    if !collection::name_exists_tx(conn, game_id, requested_name).await? {
        return Ok(requested_name.to_string());
    }

    for suffix in 2.. {
        let candidate = format!("{requested_name} ({suffix})");
        if !collection::name_exists_tx(conn, game_id, &candidate).await? {
            return Ok(candidate);
        }
    }

    unreachable!("usize suffix space is exhausted")
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
