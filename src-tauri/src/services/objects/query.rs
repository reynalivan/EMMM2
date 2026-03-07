use std::collections::HashMap;

use crate::database::object_repo::{GetObjectsResult, ObjectFilter};
use crate::services::scanner::core::normalizer::normalize_display_name;

/// Pure DB read — no filesystem access.
/// Returns objects from the DB index with `has_naming_conflict = false` (default).
/// Naming conflicts and lost object GC are handled separately by `gc_lost_objects`.
pub async fn get_filtered_objects_with_conflict_check(
    pool: &sqlx::SqlitePool,
    filter: &ObjectFilter,
) -> Result<GetObjectsResult, String> {
    let objects = crate::database::object_repo::get_filtered_objects(pool, filter)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GetObjectsResult {
        objects,
        lost_objects: vec![],
    })
}

/// Garbage-collect objects whose folders no longer exist on disk.
/// Also detects naming conflicts (enabled + disabled variants of the same name).
///
/// Called at specific sync points only:
/// - Game switch
/// - Manual "Sync Database" action
/// - Watcher `Removed` events (via lifecycle.rs)
/// - App startup
pub async fn gc_lost_objects(
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, String> {
    let filter = ObjectFilter {
        game_id: game_id.to_string(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };

    let objects = crate::database::object_repo::get_filtered_objects(pool, &filter)
        .await
        .map_err(|e| e.to_string())?;

    let mod_path_opt = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(ref mod_path) = mod_path_opt else {
        return Ok(vec![]);
    };

    let mods_dir = std::path::Path::new(mod_path);
    if !mods_dir.is_dir() {
        // mods_path gone — delete all objects for this game
        let mut lost_names = Vec::new();
        for obj in objects {
            let _ = crate::database::object_repo::delete_object(pool, &obj.id).await;
            lost_names.push(obj.name);
        }
        return Ok(lost_names);
    }

    // Build normalized folder name set from disk
    let mut norm_set: HashMap<String, Vec<String>> = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') {
                    let key = normalize_display_name(&name).to_lowercase();
                    norm_set.entry(key).or_default().push(name);
                }
            }
        }
    }

    let mut lost_names = Vec::new();
    for obj in objects {
        let key = normalize_display_name(&obj.folder_path).to_lowercase();
        if norm_set.get(&key).is_none() {
            log::info!(
                "GC: lost object '{}' (folder_path='{}') — deleting from DB",
                obj.name,
                obj.folder_path
            );
            let _ = crate::database::object_repo::delete_object(pool, &obj.id).await;
            lost_names.push(obj.name);
        }
    }

    Ok(lost_names)
}

pub async fn get_category_counts_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    safe_mode: bool,
) -> Result<Vec<crate::database::object_repo::CategoryCount>, String> {
    crate::database::object_repo::get_category_counts(pool, game_id, safe_mode)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_object_by_id_service(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, String> {
    crate::database::object_repo::get_game_object_by_id(pool, id)
        .await
        .map_err(|e| e.to_string())
}
