//! Projected-state loading and signature computation for a collection.

use crate::modules::collections::domain::collection::{
    Collection, CollectionMod, CollectionObject, ProjectedCollectionState,
};
use crate::shared::errors::CollectionError;
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::workspace::application::projected_state;
use sqlx::SqlitePool;

pub(crate) async fn load_projected_collection_state(
    pool: &SqlitePool,
    collection: &crate::modules::collections::domain::collection::Collection,
    mods_path: Option<&str>,
) -> Result<ProjectedCollectionState, CollectionError> {
    if mods_path.is_none() {
        if let Some(snapshot_json) = collection.snapshot_json.as_deref() {
            if let Some(snapshot) = projected_state::parse_snapshot_json(snapshot_json) {
                let active_root_count = snapshot.summary.active_root_count as i32;
                if collection.display_mod_count != active_root_count {
                    collection::update_display_counts(pool, &collection.id, active_root_count)
                        .await?;
                }
                return Ok(snapshot);
            }
        }
    }

    let mods = collection::get_mods(pool, &collection.id).await?;
    let objects = collection::get_objects(pool, &collection.id).await?;
    let snapshot = projected_state::build_projected_state(&mods, &objects, mods_path);
    if mods_path.is_some() {
        return Ok(snapshot);
    }

    let signature = projected_state::signature_for_projected_state(&snapshot);
    let snapshot_json = projected_state::serialize_snapshot_json(&snapshot);

    collection::update_snapshot(
        pool,
        &collection.id,
        snapshot_json.as_deref(),
        &signature,
        snapshot.summary.active_root_count as i32,
    )
    .await?;

    Ok(snapshot)
}

/// Persist canonical members plus the derived projected snapshot, signature,
/// and lightweight list count in one transaction.
pub(crate) async fn persist_projected_state<'a, A>(
    conn: A,
    collection_id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    state: &ProjectedCollectionState,
) -> Result<(), CollectionError>
where
    A: sqlx::Acquire<'a, Database = sqlx::Sqlite>,
{
    persist_projected_state_inner(conn, collection_id, mods, objects, state, true).await
}

pub(crate) async fn persist_projected_state_tx(
    conn: &mut sqlx::SqliteConnection,
    collection_id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    state: &ProjectedCollectionState,
) -> Result<(), CollectionError> {
    persist_projected_state_tx_inner(conn, collection_id, mods, objects, state, true).await
}

pub(crate) async fn refresh_projected_state_preserving_signature<'a, A>(
    conn: A,
    collection_id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    state: &ProjectedCollectionState,
) -> Result<(), CollectionError>
where
    A: sqlx::Acquire<'a, Database = sqlx::Sqlite>,
{
    persist_projected_state_inner(conn, collection_id, mods, objects, state, false).await
}

async fn persist_projected_state_inner<'a, A>(
    conn: A,
    collection_id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    state: &ProjectedCollectionState,
    update_signature: bool,
) -> Result<(), CollectionError>
where
    A: sqlx::Acquire<'a, Database = sqlx::Sqlite>,
{
    let mut tx = conn.begin().await?;
    persist_projected_state_tx_inner(
        &mut tx,
        collection_id,
        mods,
        objects,
        state,
        update_signature,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn persist_projected_state_tx_inner(
    conn: &mut sqlx::SqliteConnection,
    collection_id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    state: &ProjectedCollectionState,
    update_signature: bool,
) -> Result<(), CollectionError> {
    let signature = projected_state::signature_for_projected_state(state);
    let snapshot_json = projected_state::serialize_snapshot_json(state);

    collection::replace_all_state_tx(
        conn,
        collection::CollectionStateSnapshot {
            collection_id,
            mods,
            objects,
            signature: Some(&signature),
            update_signature,
            snapshot_json: snapshot_json.as_deref(),
            display_mod_count: state.summary.active_root_count as i32,
        },
    )
    .await?;
    Ok(())
}

pub(crate) fn collection_members_from_projected_state(
    collection_id: &str,
    state: &ProjectedCollectionState,
) -> (Vec<CollectionMod>, Vec<CollectionObject>) {
    let mods = projected_state::mods_from_projected_state(collection_id, state);
    let objects = projected_state::objects_from_projected_state(collection_id, state);
    (mods, objects)
}

pub fn compute_signature(mods: &[CollectionMod], objects: &[CollectionObject]) -> String {
    let projected_state = projected_state::build_projected_state(mods, objects, None);
    projected_state::signature_for_projected_state(&projected_state)
}

/// Load a collection or report it missing. The `get_by_id` → `NotFound`
/// pairing was previously written out at nine call sites.
pub(crate) async fn require_collection(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Collection, CollectionError> {
    collection::get_by_id(pool, collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.to_string(),
        })
}

/// Reject a collection that belongs to a different game.
pub(crate) fn require_game_match(
    collection: &Collection,
    game_id: &str,
) -> Result<(), CollectionError> {
    if collection.game_id != game_id {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to game '{}'",
            collection.id, game_id
        )));
    }
    Ok(())
}
