use super::nested_walker;
use super::root_resolution::{is_effectively_enabled_folder_path, is_foldergrid_level_mod_path};
use super::types::{CollectionObjectState, ModState};
use crate::database::{collection_repo, game_repo};
use crate::services::path_key::canonical_collection_path_key;
use sqlx::SqlitePool;
use std::collections::HashSet;

pub(crate) struct CurrentEffectiveCorridorState {
    pub effective_db_mod_ids: Vec<String>,
    pub effective_nested_paths: Vec<String>,
    pub object_states: Vec<CollectionObjectState>,
}

pub(crate) async fn resolve_current_effective_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CurrentEffectiveCorridorState, String> {
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let object_states = collection_repo::get_current_object_states_for_game(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let disabled_object_ids: HashSet<String> = object_states
        .iter()
        .filter(|state| !state.is_enabled)
        .map(|state| state.object_id.clone())
        .collect();

    let active_db_mods =
        collection_repo::get_enabled_mod_id_and_paths_for_corridor(pool, game_id, is_safe)
            .await
            .map_err(|e| e.to_string())?;
    let active_ids: Vec<String> = active_db_mods.iter().map(|(id, _)| id.clone()).collect();
    let effective_db_mod_ids = if active_ids.is_empty() {
        Vec::new()
    } else {
        collection_repo::get_mod_states_by_ids(pool, game_id, &active_ids)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|state| {
                is_current_db_mod_enabled(state, &disabled_object_ids, mods_path.as_deref())
            })
            .map(|state| state.id)
            .collect()
    };

    let effective_db_paths =
        collection_repo::get_mod_paths_for_ids_pool(pool, &effective_db_mod_ids)
            .await
            .map_err(|e| e.to_string())?;
    let active_db_path_keys: HashSet<String> = effective_db_paths
        .values()
        .filter_map(|path| canonical_collection_path_key(path, mods_path.as_deref()))
        .collect();

    let effective_nested_paths = load_enabled_nested_paths(mods_path.as_deref(), is_safe)?
        .into_iter()
        .filter(|path| {
            canonical_collection_path_key(path, mods_path.as_deref())
                .is_none_or(|key| !active_db_path_keys.contains(&key))
        })
        .collect();

    Ok(CurrentEffectiveCorridorState {
        effective_db_mod_ids,
        effective_nested_paths,
        object_states,
    })
}

fn is_collection_db_mod_target(
    state: &ModState,
    disabled_object_ids: &HashSet<String>,
    mods_path: Option<&str>,
) -> bool {
    if state
        .object_id
        .as_ref()
        .is_some_and(|object_id| disabled_object_ids.contains(object_id))
    {
        return false;
    }

    is_foldergrid_level_mod_path(&state.folder_path, mods_path)
}

fn is_current_db_mod_enabled(
    state: &ModState,
    disabled_object_ids: &HashSet<String>,
    mods_path: Option<&str>,
) -> bool {
    if !is_collection_db_mod_target(state, disabled_object_ids, mods_path) {
        return false;
    }

    is_effectively_enabled_folder_path(&state.folder_path, mods_path)
}

fn load_enabled_nested_paths(
    mods_path: Option<&str>,
    is_safe: bool,
) -> Result<Vec<String>, String> {
    let Some(mods_path) = mods_path else {
        return Ok(Vec::new());
    };

    nested_walker::walk_nested_mods(mods_path).map(|mods| {
        mods.into_iter()
            .filter(|state| state.is_enabled && state.is_safe == is_safe)
            .filter(|state| is_effectively_enabled_folder_path(&state.folder_path, Some(mods_path)))
            .map(|state| state.folder_path)
            .collect()
    })
}
