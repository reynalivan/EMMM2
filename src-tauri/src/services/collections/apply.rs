use super::types::{ApplyCollectionResult, ModState};
use crate::database::collection_repo;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use uuid::Uuid;

static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").expect("valid regex"));

pub async fn apply_collection(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    collection_id: &str,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<ApplyCollectionResult, String> {
    let (collection_name, is_safe_context): (String, bool) = if let Some(details) =
        collection_repo::get_collection_details(pool, collection_id, game_id)
            .await
            .map_err(|e| e.to_string())?
    {
        (details.collection.name, details.collection.is_safe_context)
    } else {
        return Err("Collection not found".to_string());
    };

    if safe_mode_enabled && !is_safe_context {
        return Err(
            "Collection contains non-safe context. Disable Safe Mode to apply.".to_string(),
        );
    }

    // Step 1: Get mod_ids that still exist in the mods table
    let target_ids =
        collection_repo::get_mod_ids_for_collection_in_game(pool, collection_id, game_id)
            .await
            .map_err(|e| e.to_string())?;

    // Step 2: Reconcile orphaned items by mod_path (US-8.3 fallback)
    let orphaned =
        collection_repo::get_collection_items_with_missing_mods(pool, collection_id, game_id)
            .await
            .map_err(|e| e.to_string())?;

    let mut reconciled_ids = target_ids;
    let mut reconcile_warnings = Vec::new();

    for (old_id, maybe_path) in &orphaned {
        if let Some(path) = maybe_path {
            let found = collection_repo::get_mod_id_by_path(pool, path, game_id)
                .await
                .map_err(|e| e.to_string())?;

            if let Some(new_id) = found {
                // Re-link: update collection_items to point to the new mod ID
                collection_repo::update_collection_item_mod_id(
                    pool,
                    collection_id,
                    old_id,
                    &new_id,
                )
                .await
                .map_err(|e| e.to_string())?;
                reconciled_ids.push(new_id);
            } else {
                let name = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| old_id.clone());
                reconcile_warnings.push(format!("Skipping missing mod: {}", name));
            }
        } else {
            reconcile_warnings.push(format!("Skipping orphaned mod (no path): {}", old_id));
        }
    }

    let target_ids = reconciled_ids;

    if target_ids.is_empty() {
        return Ok(ApplyCollectionResult {
            changed_count: 0,
            warnings: vec![format!("Collection '{collection_name}' has no items")],
        });
    }

    let mut states = collection_repo::get_mod_states_by_ids(pool, game_id, &target_ids)
        .await
        .map_err(|e| e.to_string())?;
    let object_ids = collection_repo::get_object_ids_for_collection(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;

    let conflicts = collection_repo::get_enabled_conflicting_mod_states(
        pool,
        game_id,
        &target_ids,
        &object_ids,
    )
    .await
    .map_err(|e| e.to_string())?;
    states.extend(conflicts);

    snapshot_current_state(pool, game_id, safe_mode_enabled).await?;

    let mut result = apply_state_change(pool, watcher_state, states, &target_ids).await?;
    result.warnings.extend(reconcile_warnings);
    Ok(result)
}

async fn snapshot_current_state(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Delete existing snapshot collection mapping and the collection itself for this game
    collection_repo::delete_snapshot_collection(&mut tx, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let currently_enabled = collection_repo::get_enabled_mod_ids(&mut tx, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let paths = collection_repo::get_mod_paths_for_ids(&mut tx, &currently_enabled)
        .await
        .unwrap_or_default();

    let snapshot_id = Uuid::new_v4().to_string();
    let name = format!("Unsaved {}", Uuid::new_v4());

    collection_repo::insert_snapshot_collection(
        &mut tx,
        &snapshot_id,
        &name,
        game_id,
        safe_mode_enabled,
    )
    .await
    .map_err(|e| e.to_string())?;

    for mod_id in currently_enabled {
        let mod_path = paths.get(&mod_id).map(|s| s.as_str());
        collection_repo::insert_collection_item(&mut tx, &snapshot_id, &mod_id, mod_path)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn apply_state_change(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    states: Vec<ModState>,
    target_ids: &[String],
) -> Result<ApplyCollectionResult, String> {
    let desired: HashMap<String, String> = states
        .iter()
        .map(|state| {
            let target = if target_ids.contains(&state.id) {
                "ENABLED"
            } else {
                "DISABLED"
            };
            (state.id.clone(), target.to_string())
        })
        .collect();

    let (changed, warnings) =
        apply_with_desired_status(pool, watcher_state, states, desired).await?;

    Ok(ApplyCollectionResult {
        changed_count: changed,
        warnings,
    })
}

async fn apply_with_desired_status(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    states: Vec<ModState>,
    desired: HashMap<String, String>,
) -> Result<(usize, Vec<String>), String> {
    let mut updates = Vec::new();
    let mut warnings = Vec::new();

    {
        let _guard = SuppressionGuard::new(&watcher_state.suppressor);
        for state in &states {
            let next_status = desired
                .get(&state.id)
                .cloned()
                .unwrap_or_else(|| state.status.clone());
            if state.status == next_status {
                continue;
            }

            let new_path = rename_for_status(&state.folder_path, next_status == "ENABLED")?;
            if let Some(path) = new_path {
                if !Path::new(&state.folder_path).exists() {
                    let folder_name = Path::new(&state.folder_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| state.id.clone());
                    warnings.push(format!("Skipping missing mod: {}", folder_name));
                    continue;
                }
                fs::rename(&state.folder_path, &path).map_err(|e| e.to_string())?;
                updates.push((state.id.clone(), next_status, path));
                continue;
            }

            updates.push((state.id.clone(), next_status, state.folder_path.clone()));
        }
    }

    collection_repo::batch_update_mods_status_and_path(pool, &updates)
        .await
        .map_err(|e| e.to_string())?;

    Ok((updates.len(), warnings))
}

fn rename_for_status(path: &str, to_enabled: bool) -> Result<Option<String>, String> {
    let old = Path::new(path);
    let parent = old.parent().ok_or("Invalid mod folder path")?;
    let name = old
        .file_name()
        .ok_or("Invalid mod folder name")?
        .to_string_lossy()
        .to_string();

    let clean_name = DISABLED_RE.replace(&name, "").trim().to_string();
    let next_name = if to_enabled {
        clean_name
    } else {
        format!("DISABLED {clean_name}")
    };

    if next_name == name {
        return Ok(None);
    }

    let next = parent.join(next_name);
    if next.exists() {
        return Err(format!("Target path already exists: {}", next.display()));
    }
    Ok(Some(next.to_string_lossy().to_string()))
}
