use sqlx::SqlitePool;

use crate::domain::errors::RuntimeStateError;
use crate::domain::runtime_state::{
    CollectionRuntimeDescriptor, CollectionRuntimeSnapshot, LastChangesSnapshot, LastChangesSource,
    RuntimeCounts, RuntimeSafetySummary, RuntimeStatus,
};
use crate::repo::{collection, collection::runtime};
use crate::services::projected_state;

/// Read the compact global collection status from one SQLite snapshot.
///
/// This deliberately does not resolve preview metadata or build a tree. The
/// Collections page keeps those heavyweight reads behind its lazy preview
/// queries, while topbar/global consumers only need this descriptor.
pub async fn get_collection_runtime_descriptor(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<CollectionRuntimeDescriptor, RuntimeStateError> {
    let mut tx = pool.begin().await?;
    let runtime = collection::runtime::get_tx(&mut tx, game_id).await?;
    let active_collection = match runtime
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
    {
        Some(id) => collection::get_by_id_tx(&mut tx, id).await?,
        None => None,
    }
    .filter(|collection| collection.game_id == game_id && !collection.is_draft);
    let live_summary =
        crate::services::collection::load_live_runtime_summary_tx(&mut tx, game_id)
            .await
            .map_err(RuntimeStateError::from)?;
    let is_current = match active_collection.as_ref() {
        Some(collection) => {
            crate::services::collection::live_runtime_matches_collection_tx(
                &mut tx,
                game_id,
                &collection.id,
            )
            .await
            .map_err(RuntimeStateError::from)?
        }
        None => false,
    };
    let missing_count = match active_collection.as_ref() {
        Some(collection) => {
            crate::services::collection::missing_collection_member_count_tx(
                &mut tx,
                game_id,
                &collection.id,
            )
            .await
            .map_err(RuntimeStateError::from)?
        }
        None => 0,
    };
    let active_collection_id = active_collection
        .as_ref()
        .map(|collection| collection.id.clone());
    let active_collection_name = active_collection
        .as_ref()
        .map(|collection| collection.name.clone());
    let runtime_status = match active_collection.as_ref() {
        Some(_) if is_current => RuntimeStatus::Clean,
        Some(_) => RuntimeStatus::Modified,
        None => RuntimeStatus::Unsaved,
    };
    let last_changes = if let Some(state) = runtime
        .as_ref()
        .filter(|state| state.draft_collection_id.is_some())
    {
        Some(LastChangesSnapshot {
            source: LastChangesSource::Draft,
            collection_id: state.draft_collection_id.clone(),
            base_collection_id: state.draft_base_collection_id.clone(),
            can_restore: true,
        })
    } else if runtime_status == RuntimeStatus::Modified
        || (runtime_status == RuntimeStatus::Unsaved && live_summary.active_mod_count > 0)
    {
        Some(LastChangesSnapshot {
            source: LastChangesSource::Live,
            collection_id: None,
            base_collection_id: active_collection_id.clone(),
            can_restore: false,
        })
    } else {
        None
    };

    tx.commit().await?;

    Ok(CollectionRuntimeDescriptor {
        game_id: game_id.to_string(),
        active_collection_id,
        active_collection_name,
        runtime_status,
        missing_count,
        safety: RuntimeSafetySummary {
            is_safe: live_summary.is_safe,
            is_safety_classified: live_summary.is_safety_classified,
        },
        counts: RuntimeCounts {
            active_mod_count: live_summary.active_mod_count,
            object_count: live_summary.object_count,
            enabled_object_count: live_summary.enabled_object_count,
        },
        last_changes,
    })
}

pub async fn get_collection_runtime_state(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<CollectionRuntimeSnapshot, RuntimeStateError> {
    let mods_path = crate::services::collection::load_game_mods_path(pool, game_id)
        .await
        .map_err(RuntimeStateError::from)?;
    let (current_mods, current_objects) =
        crate::services::collection::load_live_runtime_state(pool, game_id)
            .await
            .map_err(RuntimeStateError::from)?;
    let is_safe = crate::services::collection::live_runtime_is_safe(pool, game_id)
        .await
        .map_err(RuntimeStateError::from)?;
    let is_safety_classified = current_mods.iter().all(|member| {
        member
            .safety_source
            .as_deref()
            .is_some_and(|source| source != crate::common::safety_constants::SAFETY_SOURCE_UNKNOWN)
    });
    let projected_state = projected_state::build_projected_state(
        &current_mods,
        &current_objects,
        mods_path.as_deref(),
    );
    let current_tree_nodes =
        projected_state::build_preview_tree_from_projected_state(&projected_state);
    let current_signature =
        projected_state::signature_for_projected_state(&projected_state);
    let runtime = collection::runtime::get(pool, game_id).await?;
    let active_collection = match runtime
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
    {
        Some(id) => collection::get_by_id(pool, id).await?,
        None => None,
    }
    .filter(|collection| collection.game_id == game_id && !collection.is_draft);
    let active_collection_id = active_collection
        .as_ref()
        .map(|collection| collection.id.clone());
    let active_collection_name = active_collection
        .as_ref()
        .map(|collection| collection.name.clone());
    let is_dirty = active_collection.as_ref().is_none_or(|collection| {
        collection.signature.as_deref() != Some(current_signature.as_str())
    });
    let runtime_status = match active_collection.as_ref() {
        Some(_) if is_dirty => RuntimeStatus::Modified,
        Some(_) => RuntimeStatus::Clean,
        None => RuntimeStatus::Unsaved,
    };
    let missing_count = match active_collection.as_ref() {
        Some(collection) => {
            crate::services::collection::load_projected_collection_state(
                pool,
                collection,
                mods_path.as_deref(),
            )
            .await
            .map_err(RuntimeStateError::from)?
            .summary
            .missing_root_count
        }
        None => 0,
    };
    let last_changes = if let Some(state) = runtime
        .as_ref()
        .filter(|state| state.draft_collection_id.is_some())
    {
        Some(LastChangesSnapshot {
            source: LastChangesSource::Draft,
            collection_id: state.draft_collection_id.clone(),
            base_collection_id: state.draft_base_collection_id.clone(),
            can_restore: true,
        })
    } else if is_dirty
        && (runtime_status == RuntimeStatus::Modified
            || projected_state.summary.active_root_count > 0)
    {
        Some(LastChangesSnapshot {
            source: LastChangesSource::Live,
            collection_id: None,
            base_collection_id: active_collection_id.clone(),
            can_restore: false,
        })
    } else {
        None
    };

    Ok(CollectionRuntimeSnapshot {
        game_id: game_id.to_string(),
        active_collection_id,
        active_collection_name,
        current_signature,
        is_dirty,
        runtime_status,
        is_safe,
        is_safety_classified,
        missing_count,
        last_changes,
        current_mods,
        current_objects,
        current_tree_nodes,
        projected_state,
    })
}

#[cfg(test)]
mod tests;
