use sqlx::SqlitePool;

use super::live_state::{live_runtime_is_safe, load_game_mods_path, load_live_runtime_state};
use super::projection::persist_projected_state;
use crate::domain::collection::{CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, collection_runtime_repo};
use crate::services::projected_state_service;

pub async fn capture_last_changes_if_needed(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, CollectionError> {
    let runtime = collection_runtime_repo::get(pool, game_id).await?;
    let (mods, objects) = load_live_runtime_state(pool, game_id).await?;
    let runtime_is_empty = mods.is_empty() && objects.is_empty();
    let state = projected_state_service::build_projected_state(
        &mods,
        &objects,
        load_game_mods_path(pool, game_id).await?.as_deref(),
    );
    if runtime
        .as_ref()
        .and_then(|value| value.active_collection_id.as_ref())
        .is_none()
        && runtime_is_empty
    {
        return Ok(None);
    }

    let signature = projected_state_service::signature_for_projected_state(&state);
    let base_collection_id = runtime
        .as_ref()
        .and_then(|value| value.active_collection_id.clone());
    if let Some(base) = base_collection_id.as_deref() {
        if collection_repo::get_by_id(pool, base)
            .await?
            .is_some_and(|collection| collection.signature.as_deref() == Some(signature.as_str()))
        {
            return Ok(None);
        }
    }

    let existing_draft_id = match runtime
        .as_ref()
        .and_then(|value| value.draft_collection_id.clone())
    {
        Some(id) if collection_repo::get_by_id(pool, &id).await?.is_some() => Some(id),
        _ => None,
    };
    let draft_id = existing_draft_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let is_safe = live_runtime_is_safe(pool, game_id).await?;
    let persisted_mods = mods
        .into_iter()
        .map(|member| CollectionMod {
            collection_id: draft_id.clone(),
            ..member
        })
        .collect::<Vec<_>>();
    let persisted_objects = objects
        .into_iter()
        .map(|member| CollectionObject {
            collection_id: draft_id.clone(),
            ..member
        })
        .collect::<Vec<_>>();

    let mut tx = pool.begin().await?;
    if existing_draft_id.is_none() {
        collection_repo::delete_unsaved_for_game_tx(&mut tx, game_id).await?;
        let internal_name = format!("__last_changes__{draft_id}");
        collection_repo::create_tx(
            &mut tx,
            collection_repo::CreateCollectionRow {
                id: &draft_id,
                game_id,
                name: &internal_name,
                is_safe,
            },
        )
        .await?;
    } else {
        collection_repo::update_safety_summary_tx(&mut tx, &draft_id, is_safe).await?;
    }
    persist_projected_state(
        &mut *tx,
        &draft_id,
        &persisted_mods,
        &persisted_objects,
        &state,
    )
    .await?;
    collection_runtime_repo::set_draft_tx(
        &mut tx,
        game_id,
        &draft_id,
        base_collection_id.as_deref(),
    )
    .await?;
    tx.commit().await?;

    Ok(Some(draft_id))
}

pub async fn clear_last_changes(pool: &SqlitePool, game_id: &str) -> Result<(), CollectionError> {
    let mut tx = pool.begin().await?;
    let draft_id = collection_runtime_repo::get_tx(&mut tx, game_id)
        .await?
        .and_then(|value| value.draft_collection_id);
    collection_runtime_repo::clear_draft_tx(&mut tx, game_id).await?;
    if let Some(draft_id) = draft_id {
        ensure_rollback_draft_is_unreferenced(&mut tx, &draft_id).await?;
        collection_repo::delete_tx(&mut tx, &draft_id).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn ensure_rollback_draft_is_unreferenced(
    conn: &mut sqlx::SqliteConnection,
    draft_id: &str,
) -> Result<(), CollectionError> {
    if crate::repo::task_repo::open_task_references_rollback_collection_tx(conn, draft_id).await? {
        return Err(CollectionError::Validation(
            "Last changes is required by an open recovery task; resolve recovery before clearing or saving it"
                .to_string(),
        ));
    }
    Ok(())
}

/// Resolve a draft's baseline only when it is still a named collection for
/// the same game. Hidden draft rows are never valid active baselines.
pub async fn valid_active_baseline(
    pool: &SqlitePool,
    game_id: &str,
    candidate_id: Option<&str>,
) -> Result<Option<String>, CollectionError> {
    let Some(candidate_id) = candidate_id else {
        return Ok(None);
    };
    Ok(collection_repo::get_by_id(pool, candidate_id)
        .await?
        .filter(|collection| collection.game_id == game_id && !collection.is_draft)
        .map(|collection| collection.id))
}
