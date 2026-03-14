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
///
/// Compares each object's `folder_path` (a folder name like `"Alhaitham"`)
/// against the normalized set of filesystem directories under `mods_path`.
/// Objects with no matching FS directory are deleted from the DB.
///
/// # Safety Invariants
/// - If `mods_dir` doesn't exist → GC is skipped (config issue, not GC signal).
/// - If filesystem returns 0 non-hidden folders → GC is skipped (FS unreadable).
/// - If GC would delete ALL objects for a game → GC is **aborted** entirely
///   (likely a `folder_path` format mismatch, not legitimate cleanup).
///
/// # Call Sites
/// - `startup_sync::reconcile_game` (app startup)
/// - Manual "Sync Database" action
/// - Watcher `Removed` events (via lifecycle.rs)
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

    if objects.is_empty() {
        return Ok(vec![]);
    }

    let mod_path_opt = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(ref mod_path) = mod_path_opt else {
        return Ok(vec![]);
    };

    let mods_dir = std::path::Path::new(mod_path);
    if !mods_dir.is_dir() {
        // mods_path gone — do NOT delete objects; this is a config issue, not a GC signal.
        log::warn!(
            "GC skipped for game '{}': mods_dir '{}' does not exist. Keeping DB intact.",
            game_id,
            mod_path
        );
        return Ok(vec![]);
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

    // SAFETY: If FS has 0 folders, do NOT GC — filesystem is likely unreadable
    if norm_set.is_empty() {
        log::warn!(
            "GC skipped for game '{}': filesystem returned 0 non-hidden folders at '{}'.",
            game_id,
            mod_path
        );
        return Ok(vec![]);
    }

    // Phase 1: Collect candidates (do NOT delete yet)
    let mut candidates: Vec<(String, String, String)> = Vec::new(); // (id, name, folder_path)
    for obj in &objects {
        let key = normalize_display_name(&obj.folder_path).to_lowercase();
        if !norm_set.contains_key(&key) {
            candidates.push((obj.id.clone(), obj.name.clone(), obj.folder_path.clone()));
        }
    }

    // SAFETY: If GC would delete ALL objects, abort — this is a bug, not cleanup
    if !candidates.is_empty() && candidates.len() == objects.len() {
        log::error!(
            "GC ABORTED for game '{}': would delete ALL {} objects. \
             This indicates a folder_path format mismatch between DB and FS, \
             not legitimate cleanup. DB objects kept intact.",
            game_id,
            objects.len()
        );
        return Ok(vec![]);
    }

    // Phase 2: Delete only the safe subset
    let mut lost_names = Vec::new();
    for (id, name, folder_path) in &candidates {
        log::info!(
            "GC: lost object '{}' (folder_path='{}') — deleting from DB",
            name,
            folder_path
        );
        let _ = crate::database::object_repo::delete_object(pool, id).await;
        lost_names.push(name.clone());
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
