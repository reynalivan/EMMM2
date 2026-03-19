use super::types::{
    Collection, CollectionDetails, CollectionObjectState, CreateCollectionInput,
    UpdateCollectionInput,
};
use crate::database::collection_repo;
use crate::database::game_repo;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<Vec<Collection>, String> {
    let _ = super::materialize_game_collections_if_missing(pool, game_id).await?;
    collection_repo::list_collections(pool, game_id, safe_mode_enabled)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_collection(
    pool: &SqlitePool,
    input: CreateCollectionInput,
) -> Result<CollectionDetails, String> {
    let auto_snapshot = input.auto_snapshot.unwrap_or(false);
    let id = Uuid::new_v4().to_string();
    let current_effective_state = if auto_snapshot {
        Some(
            super::resolve_current_effective_corridor_state(
                pool,
                &input.game_id,
                input.is_safe_context,
            )
            .await?,
        )
    } else {
        None
    };
    let current_runtime_snapshot = if auto_snapshot {
        Some(
            super::resolve_corridor_runtime_snapshot(pool, &input.game_id, input.is_safe_context)
                .await?,
        )
    } else {
        None
    };

    let (mods_path, current_object_states) = tokio::try_join!(
        game_repo::get_mod_path(pool, &input.game_id),
        collection_repo::get_current_object_states_for_game(pool, &input.game_id),
    )
    .map_err(|e| format!("Failed to prepare collection snapshot: {e}"))?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let name_trimmed = input.name.trim();

    let exists = collection_repo::check_collection_exists(
        &mut tx,
        &input.game_id,
        name_trimmed,
        input.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;

    if exists {
        return Err(format!(
            "A collection named '{}' already exists.",
            name_trimmed
        ));
    }

    collection_repo::insert_collection(
        &mut tx,
        &id,
        name_trimmed,
        &input.game_id,
        input.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut mod_ids = input.mod_ids;
    if auto_snapshot {
        mod_ids = current_effective_state
            .as_ref()
            .map(|state| state.effective_db_mod_ids.clone())
            .unwrap_or_default();
    }

    let mod_ids = unique_mod_ids(mod_ids);

    let object_states = match input.object_states {
        Some(states) => normalize_object_states(states, &current_object_states),
        None if auto_snapshot => current_effective_state
            .as_ref()
            .map(|state| state.object_states.clone())
            .unwrap_or_else(|| current_object_states.clone()),
        None => enable_all_object_states(&current_object_states),
    };

    collection_repo::batch_insert_collection_object_states(&mut tx, &id, &object_states)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let roots = if let Some(runtime_snapshot) = current_runtime_snapshot.as_ref() {
        runtime_snapshot.roots.clone()
    } else {
        build_runtime_roots_from_mod_ids(pool, &input.game_id, input.is_safe_context, &mod_ids)
            .await?
    };
    let runtime_object_states = if let Some(runtime_snapshot) = current_runtime_snapshot.as_ref() {
        runtime_snapshot.object_states.clone()
    } else {
        super::runtime_snapshot::build_runtime_object_states_for_game(
            pool,
            &input.game_id,
            &object_states,
        )
        .await?
    };

    // Execute metadata updates asynchronously in parallel outside transaction
    let info_json_paths: Vec<String> = collection_repo::get_mod_paths_for_ids_pool(pool, &mod_ids)
        .await
        .map_err(|e| e.to_string())?
        .into_values()
        .collect();
    if !info_json_paths.is_empty() {
        use crate::services::mods::info_json::{batch_update_info_jsons, ModInfoUpdate};
        let update = ModInfoUpdate {
            preset_name_add: Some(vec![name_trimmed.to_string()]),
            ..Default::default()
        };
        let _ = batch_update_info_jsons(info_json_paths, update).await;
    }

    // When saving a snapshot of the current state as a named collection,
    // that collection IS the current state — update corridor_state accordingly
    // so the frontend can show the correct "active" collection label.
    if auto_snapshot {
        if let Err(e) = crate::database::corridor_state_repo::update_active_collection_id(
            pool,
            &input.game_id,
            input.is_safe_context,
            Some(&id),
        )
        .await
        {
            log::warn!("Failed to update corridor_state.active after create_collection: {e}");
        }
    }

    super::persist_collection_runtime_materialization(
        pool,
        &id,
        &roots,
        &runtime_object_states,
        mods_path.as_deref(),
    )
    .await?;

    Ok(CollectionDetails {
        collection: Collection {
            id,
            name: input.name.trim().to_string(),
            game_id: input.game_id,
            is_safe_context: input.is_safe_context,
            member_count: roots.len(),
            is_last_unsaved: false,
        },
        mod_ids,
        object_states,
    })
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionDetails, String> {
    let (current_object_states, existing_object_states) = tokio::try_join!(
        collection_repo::get_current_object_states_for_game(pool, &input.game_id),
        collection_repo::get_collection_object_states(pool, &input.id),
    )
    .map_err(|e| format!("Failed to prepare collection update: {e}"))?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let old_name: String = collection_repo::get_collection_name(&mut tx, &input.id, &input.game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Collection not found")?;
    let current_safe_context: bool =
        sqlx::query_scalar("SELECT is_safe_context FROM collections WHERE id = ? AND game_id = ?")
            .bind(&input.id)
            .bind(&input.game_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

    let mut new_name = old_name.clone();

    // Buffers for deferred JSON batching outside the SQL transaction
    let mut info_json_renames = Vec::new();
    let mut info_json_removes = Vec::new();
    let mut info_json_adds = Vec::new();

    if let Some(name) = input.name.as_ref() {
        let name_trimmed = name.trim();
        new_name = name_trimmed.to_string();

        // Guard: check for duplicate name within same corridor
        if new_name != old_name {
            let effective_safe = input.is_safe_context.unwrap_or(current_safe_context);

            let exists = collection_repo::check_collection_exists(
                &mut tx,
                &input.game_id,
                name_trimmed,
                effective_safe,
            )
            .await
            .map_err(|e| e.to_string())?;

            if exists {
                return Err(format!(
                    "A collection named '{}' already exists.",
                    name_trimmed
                ));
            }

            info_json_renames =
                collection_repo::get_collection_root_mod_paths(pool, &input.id, &input.game_id)
                    .await
                    .map_err(|e| e.to_string())?;
        }

        collection_repo::update_collection_name(&mut tx, &input.id, &input.game_id, name_trimmed)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(safe) = input.is_safe_context {
        collection_repo::update_collection_safe_context(&mut tx, &input.id, &input.game_id, safe)
            .await
            .map_err(|e| e.to_string())?;
    }

    let next_mod_ids = input
        .mod_ids
        .as_ref()
        .map(|mod_ids| unique_mod_ids(mod_ids.clone()));
    let old_root_paths = if next_mod_ids.is_some() {
        collection_repo::get_collection_root_mod_paths(pool, &input.id, &input.game_id)
            .await
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    if let Some(object_states) = input.object_states.as_ref() {
        let normalized = normalize_object_states(object_states.clone(), &current_object_states);
        collection_repo::delete_collection_object_states(&mut tx, &input.id)
            .await
            .map_err(|e| e.to_string())?;
        collection_repo::batch_insert_collection_object_states(&mut tx, &input.id, &normalized)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    let final_object_states = if let Some(object_states) = input.object_states.as_ref() {
        normalize_object_states(object_states.clone(), &current_object_states)
    } else if existing_object_states.is_empty() {
        enable_all_object_states(&current_object_states)
    } else {
        normalize_object_states(existing_object_states.clone(), &current_object_states)
    };

    let collection_safe_context = input.is_safe_context.unwrap_or(current_safe_context);

    let roots = if let Some(mod_ids) = next_mod_ids.as_ref() {
        let new_paths = collection_repo::get_mod_paths_for_ids_pool(pool, mod_ids)
            .await
            .map_err(|e| e.to_string())?;
        let old_path_set: HashSet<String> = old_root_paths.iter().cloned().collect();
        let new_path_set: HashSet<String> = new_paths.values().cloned().collect();

        info_json_removes = old_path_set.difference(&new_path_set).cloned().collect();
        info_json_adds = new_path_set.difference(&old_path_set).cloned().collect();

        build_runtime_roots_from_mod_ids(pool, &input.game_id, collection_safe_context, mod_ids)
            .await?
    } else {
        let mut existing_roots = collection_repo::get_collection_roots(pool, &input.id)
            .await
            .map_err(|e| e.to_string())?;
        for root in &mut existing_roots {
            root.is_safe = collection_safe_context;
        }
        existing_roots
    };

    let runtime_object_states = super::runtime_snapshot::build_runtime_object_states_for_game(
        pool,
        &input.game_id,
        &final_object_states,
    )
    .await?;
    let mods_path = game_repo::get_mod_path(pool, &input.game_id)
        .await
        .map_err(|e| e.to_string())?;
    super::persist_collection_runtime_materialization(
        pool,
        &input.id,
        &roots,
        &runtime_object_states,
        mods_path.as_deref(),
    )
    .await?;

    // Execute metadata updates asynchronously in parallel after DB commit
    use crate::services::mods::info_json::{batch_update_info_jsons, ModInfoUpdate};

    if !info_json_renames.is_empty() {
        let update = ModInfoUpdate {
            preset_name_remove: Some(vec![old_name.clone()]),
            preset_name_add: Some(vec![new_name.clone()]),
            ..Default::default()
        };
        let _ = batch_update_info_jsons(info_json_renames, update).await;
    }

    if !info_json_removes.is_empty() {
        let update_rem = ModInfoUpdate {
            preset_name_remove: Some(vec![new_name.clone()]),
            ..Default::default()
        };
        let _ = batch_update_info_jsons(info_json_removes, update_rem).await;
    }

    if !info_json_adds.is_empty() {
        let update_add = ModInfoUpdate {
            preset_name_add: Some(vec![new_name.clone()]),
            ..Default::default()
        };
        let _ = batch_update_info_jsons(info_json_adds, update_add).await;
    }

    get_collection(pool, &input.id, &input.game_id).await
}

pub async fn save_snapshot_collection_as_named(
    pool: &SqlitePool,
    source_collection_id: &str,
    game_id: &str,
    name: &str,
) -> Result<CollectionDetails, String> {
    let source_details = get_collection(pool, source_collection_id, game_id).await?;
    if !source_details.collection.is_last_unsaved {
        return Err("Only the last unsaved snapshot can be saved as a named collection.".into());
    }

    let source_runtime_preview =
        super::get_collection_runtime_preview(pool, source_collection_id, game_id).await?;
    let current_corridor_state = crate::database::corridor_state_repo::get_corridor_state(
        pool,
        game_id,
        source_details.collection.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;
    let current_object_states = collection_repo::get_current_object_states_for_game(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let name_trimmed = name.trim();

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let exists = collection_repo::check_collection_exists(
        &mut tx,
        game_id,
        name_trimmed,
        source_details.collection.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;
    if exists {
        return Err(format!(
            "A collection named '{}' already exists.",
            name_trimmed
        ));
    }

    collection_repo::insert_collection(
        &mut tx,
        &id,
        name_trimmed,
        game_id,
        source_details.collection.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;

    let object_states =
        normalize_object_states(source_details.object_states.clone(), &current_object_states);
    collection_repo::batch_insert_collection_object_states(&mut tx, &id, &object_states)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let info_json_paths =
        collection_repo::get_collection_root_mod_paths(pool, source_collection_id, game_id)
            .await
            .map_err(|e| e.to_string())?;
    if !info_json_paths.is_empty() {
        use crate::services::mods::info_json::{batch_update_info_jsons, ModInfoUpdate};
        let update = ModInfoUpdate {
            preset_name_add: Some(vec![name_trimmed.to_string()]),
            ..Default::default()
        };
        let _ = batch_update_info_jsons(info_json_paths, update).await;
    }

    if current_corridor_state.active_collection_id.as_deref() == Some(source_collection_id) {
        if let Err(e) = crate::database::corridor_state_repo::update_active_collection_id(
            pool,
            game_id,
            source_details.collection.is_safe_context,
            Some(&id),
        )
        .await
        {
            log::warn!(
                "Failed to update corridor_state.active after save_snapshot_collection_as_named: {e}"
            );
        }
    }

    let runtime_object_states = super::runtime_snapshot::build_runtime_object_states_for_game(
        pool,
        game_id,
        &object_states,
    )
    .await?;
    super::persist_collection_runtime_materialization(
        pool,
        &id,
        &source_runtime_preview.roots,
        &runtime_object_states,
        mods_path.as_deref(),
    )
    .await?;

    Ok(CollectionDetails {
        collection: Collection {
            id,
            name: name_trimmed.to_string(),
            game_id: game_id.to_string(),
            is_safe_context: source_details.collection.is_safe_context,
            member_count: source_runtime_preview.roots.len(),
            is_last_unsaved: false,
        },
        mod_ids: source_details.mod_ids,
        object_states,
    })
}

pub async fn delete_collection(pool: &SqlitePool, id: &str, game_id: &str) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let name = collection_repo::get_collection_name(&mut tx, id, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Collection not found")?;
    let paths_to_remove = collection_repo::get_collection_root_mod_paths(pool, id, game_id)
        .await
        .unwrap_or_default();

    collection_repo::delete_collection(&mut tx, id, game_id)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    if !paths_to_remove.is_empty() {
        use crate::services::mods::info_json::{batch_update_info_jsons, ModInfoUpdate};
        let update = ModInfoUpdate {
            preset_name_remove: Some(vec![name.clone()]),
            ..Default::default()
        };
        let _ = batch_update_info_jsons(paths_to_remove, update).await;
    }

    Ok(())
}

async fn build_runtime_roots_from_mod_ids(
    pool: &SqlitePool,
    game_id: &str,
    is_safe_context: bool,
    mod_ids: &[String],
) -> Result<Vec<super::types::CollectionPreviewMod>, String> {
    if mod_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let roots = collection_repo::get_collection_preview_mods_by_ids(pool, game_id, mod_ids)
        .await
        .map_err(|e| e.to_string())?;
    Ok(roots
        .into_iter()
        .filter(|root| {
            super::root_resolution::is_foldergrid_level_mod_path(
                &root.folder_path,
                mods_path.as_deref(),
            ) && (!is_safe_context || root.is_safe)
        })
        .collect())
}

fn unique_mod_ids(mod_ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    mod_ids
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

fn normalize_object_states(
    mut object_states: Vec<CollectionObjectState>,
    current_object_states: &[CollectionObjectState],
) -> Vec<CollectionObjectState> {
    let mut dedup = HashMap::new();
    for state in object_states.drain(..) {
        dedup.insert(state.object_id, state.is_enabled);
    }

    let mut normalized: Vec<CollectionObjectState> = current_object_states
        .iter()
        .map(|state| CollectionObjectState {
            object_id: state.object_id.clone(),
            is_enabled: dedup
                .get(&state.object_id)
                .copied()
                .unwrap_or(state.is_enabled),
        })
        .collect();

    normalized.sort_by(|a, b| a.object_id.cmp(&b.object_id));
    normalized
}

fn enable_all_object_states(
    current_object_states: &[CollectionObjectState],
) -> Vec<CollectionObjectState> {
    current_object_states
        .iter()
        .map(|state| CollectionObjectState {
            object_id: state.object_id.clone(),
            is_enabled: true,
        })
        .collect()
}

async fn get_collection(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<CollectionDetails, String> {
    super::ensure_collection_runtime_materialized(pool, id, game_id).await?;
    let current_object_states = collection_repo::get_current_object_states_for_game(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut details = collection_repo::get_collection_details(pool, id, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Collection not found".to_string())?;
    details.object_states = if current_object_states.is_empty() {
        details.object_states
    } else if details.object_states.is_empty() {
        enable_all_object_states(&current_object_states)
    } else {
        normalize_object_states(details.object_states, &current_object_states)
    };
    Ok(details)
}
