//! Live (unsaved) runtime state read from the DB and shaped as collection members.

use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::collections::application::collection::preview_tree::resolve_preview_terminal_metadata;
use crate::modules::collections::domain::collection::{CollectionMod, CollectionObject};
use crate::modules::workspace::domain::normalizer::is_disabled_folder;
use crate::shared::errors::CollectionError;
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashMap;
use std::path::Path;

fn is_object_enabled(path_key: Option<&str>) -> bool {
    let Some(path_key) = path_key else {
        return true;
    };

    !path_key
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(is_disabled_folder)
}

/// `mods.status` records a terminal folder's own prefix. A row can therefore
/// retain status=enabled while an ancestor is disabled; only the stored path
/// describes whether it participates in the live runtime.
pub(crate) fn is_mod_effectively_active(mod_path: &str) -> bool {
    !mod_path
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(is_disabled_folder)
}

pub(crate) async fn load_live_runtime_state(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<(Vec<CollectionMod>, Vec<CollectionObject>), CollectionError> {
    let mods_path = load_game_mods_path(pool, game_id).await?;
    let current_objects = collection::get_live_objects(pool, game_id).await?;
    let current_objects: Vec<CollectionObject> = current_objects
        .into_iter()
        .map(|object| CollectionObject {
            is_enabled: is_object_enabled(object.path_key.as_deref()),
            ..object
        })
        .collect();
    let current_objects_by_id: HashMap<&str, &CollectionObject> = current_objects
        .iter()
        .map(|object| (object.object_id.as_str(), object))
        .collect();
    let current_mod_rows = collection::get_live_active_mod_rows(pool, game_id).await?;

    let mut current_mods = Vec::with_capacity(current_mod_rows.len());
    for row in current_mod_rows {
        let mod_id = row.mod_id;
        let mod_path = row.mod_path;
        let mod_path_key = row.mod_path_key;
        let object_id = row.object_id;
        let display_name = row.display_name;
        let is_safe = row.is_safe;
        let safety_source = row.safety_source;
        let preview_object = current_objects_by_id.get(object_id.as_str()).copied();
        let preview_seed = CollectionMod {
            kind: crate::modules::collections::domain::collection::MemberKind::Mod,
            collection_id: String::new(),
            mod_id: Some(mod_id.clone()),
            mod_path: mod_path.clone(),
            mod_path_key: Some(mod_path_key.clone()),
            object_id: object_id.clone(),
            display_name: Some(display_name.clone()),
            preview_path: None,
            node_type: None,
            warnings: Vec::new(),
            is_enabled: true,
            is_safe,
            safety_source: safety_source.clone(),
        };
        let preview_metadata =
            resolve_preview_terminal_metadata(preview_object, &preview_seed, mods_path.as_deref());

        current_mods.push(CollectionMod {
            kind: crate::modules::collections::domain::collection::MemberKind::Mod,
            collection_id: String::new(),
            mod_id: Some(mod_id),
            mod_path,
            mod_path_key: Some(mod_path_key),
            object_id,
            display_name: Some(display_name),
            preview_path: preview_metadata.preview_path,
            node_type: preview_metadata.node_type,
            warnings: preview_metadata.warnings,
            is_enabled: true,
            is_safe,
            safety_source,
        });
    }

    Ok((current_mods, current_objects))
}

/// Absolute roots of the active mods that a current-runtime snapshot records.
///
/// Folder-conflict candidates use filesystem paths, while `mods.folder_path`
/// is stored relative to the configured Mods root.
pub(crate) async fn active_runtime_snapshot_scope_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, CollectionError> {
    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| {
            CollectionError::Validation(format!("Game '{game_id}' has no configured Mods path"))
        })?;
    let mods_root = Path::new(&mods_path);
    let active_paths =
        crate::modules::library::adapters::sqlite::mods::get_enabled_mods_paths(pool, game_id)
            .await?;

    Ok(active_paths
        .into_iter()
        .map(|path| path.resolve(mods_root).to_string_lossy().to_string())
        .collect())
}

pub(crate) async fn live_runtime_is_safe(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<bool, CollectionError> {
    let active_mods = collection::get_live_active_mod_rows(pool, game_id).await?;
    Ok(active_mods
        .iter()
        .filter(|member| is_mod_effectively_active(&member.mod_path))
        .all(|member| member.is_safe))
}

pub(crate) async fn load_game_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, CollectionError> {
    Ok(
        crate::modules::games::adapters::sqlite::game::get_configured_mods_path(pool, game_id)
            .await?,
    )
}

pub(crate) struct LiveRuntimeSummary {
    pub active_mod_count: usize,
    pub object_count: usize,
    pub enabled_object_count: usize,
    pub is_safe: bool,
    pub is_safety_classified: bool,
}

pub(crate) async fn load_live_runtime_summary_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<LiveRuntimeSummary, CollectionError> {
    let objects = collection::get_live_objects_tx(conn, game_id).await?;
    let active_mods = collection::get_live_active_mod_rows_tx(conn, game_id).await?;
    let effectively_active_mods = active_mods
        .iter()
        .filter(|member| is_mod_effectively_active(&member.mod_path))
        .collect::<Vec<_>>();
    let is_safety_classified = effectively_active_mods.iter().all(|member| {
        member
            .safety_source
            .as_deref()
            .is_some_and(|source| source != crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN)
    });

    Ok(LiveRuntimeSummary {
        active_mod_count: effectively_active_mods.len(),
        object_count: objects.len(),
        enabled_object_count: objects
            .iter()
            .filter(|object| is_object_enabled(object.path_key.as_deref()))
            .count(),
        is_safe: effectively_active_mods.iter().all(|member| member.is_safe),
        is_safety_classified,
    })
}

pub(crate) async fn live_runtime_matches_collection_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    collection_id: &str,
) -> Result<bool, CollectionError> {
    let live_mods = collection::get_live_active_mod_rows_tx(conn, game_id).await?;
    let collection_members =
        collection::get_runtime_collection_membership_tx(conn, collection_id).await?;
    let live_mod_paths = live_mods
        .iter()
        .map(|member| member.mod_path_key.as_str())
        .collect::<std::collections::HashSet<_>>();
    let collection_mod_paths = collection_members
        .mod_path_keys
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    if live_mod_paths != collection_mod_paths {
        return Ok(false);
    }

    let live_objects = collection::get_live_objects_tx(conn, game_id).await?;
    let live_object_states = live_objects
        .into_iter()
        .filter_map(|object| {
            object
                .path_key
                .map(|path_key| (path_key.clone(), is_object_enabled(Some(path_key.as_str()))))
        })
        .collect::<HashMap<_, _>>();
    let collection_object_states = collection_members
        .object_states
        .into_iter()
        .collect::<HashMap<_, _>>();

    Ok(live_object_states == collection_object_states)
}

pub(crate) async fn missing_collection_member_count_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    collection_id: &str,
) -> Result<usize, CollectionError> {
    crate::modules::collections::adapters::sqlite::count_missing_mods_tx(
        conn,
        game_id,
        collection_id,
    )
    .await
}
