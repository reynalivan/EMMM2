//! Live (unsaved) runtime state read from the DB and shaped as collection members.

use crate::common::normalizer::is_disabled_folder;
use crate::domain::collection::{CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;
use crate::repo::collection_repo;
use crate::services::collection_preview_tree::resolve_preview_terminal_metadata;
use sqlx::SqlitePool;

fn is_object_enabled(path_key: Option<&str>) -> bool {
    let Some(path_key) = path_key else {
        return true;
    };

    !path_key
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(is_disabled_folder)
}

pub(crate) async fn load_live_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<(Vec<CollectionMod>, Vec<CollectionObject>), CollectionError> {
    load_live_state(pool, game_id, Some(is_safe)).await
}

pub(crate) async fn load_live_runtime_state(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<(Vec<CollectionMod>, Vec<CollectionObject>), CollectionError> {
    load_live_state(pool, game_id, None).await
}

async fn load_live_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: Option<bool>,
) -> Result<(Vec<CollectionMod>, Vec<CollectionObject>), CollectionError> {
    let mods_path = load_game_mods_path(pool, game_id).await?;
    let current_objects = collection_repo::get_live_objects(pool, game_id).await?;
    let current_objects: Vec<CollectionObject> = current_objects
        .into_iter()
        .map(|object| CollectionObject {
            is_enabled: is_object_enabled(object.path_key.as_deref()),
            ..object
        })
        .collect();
    let current_mod_rows =
        collection_repo::get_live_active_mod_rows(pool, game_id, is_safe).await?;

    let mut current_mods = Vec::with_capacity(current_mod_rows.len());
    for row in current_mod_rows {
        let mod_id = row.mod_id;
        let mod_path = row.mod_path;
        let mod_path_key = row.mod_path_key;
        let object_id = row.object_id;
        let display_name = row.display_name;
        let preview_object = current_objects
            .iter()
            .find(|object| object.object_id == object_id);
        let preview_seed = CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
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
        };
        let preview_metadata =
            resolve_preview_terminal_metadata(preview_object, &preview_seed, mods_path.as_deref());

        current_mods.push(CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
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
        });
    }

    Ok((current_mods, current_objects))
}

pub(super) async fn live_runtime_is_safe(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<bool, CollectionError> {
    let unsafe_count = crate::repo::mod_repo::count_active_unsafe_mods(pool, game_id).await?;

    Ok(unsafe_count == 0)
}

pub(super) async fn load_game_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, CollectionError> {
    Ok(crate::repo::game_repo::get_configured_mods_path(pool, game_id).await?)
}
