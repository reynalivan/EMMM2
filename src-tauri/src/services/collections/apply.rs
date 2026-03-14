use super::nested_walker;
use super::types::{ApplyCollectionResult, ModState};
use crate::database::collection_repo;
use crate::database::game_repo;
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
    let (_, is_safe_context): (String, bool) = if let Some(details) =
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
            "Collection contains non-safe mods. Disable Privacy Mode to apply.".to_string(),
        );
    }

    if !safe_mode_enabled && is_safe_context {
        return Err("Collection is Safe. Enable Privacy Mode to apply.".to_string());
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

    let nested_target_paths = collection_repo::get_nested_collection_items(pool, collection_id)
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

    // An empty collection is a valid state (Disable All Mods).
    // We let it proceed with an empty target_ids set to clear the loadout.

    // Gather ALL enabled mods + collection targets (same pattern as undo_collection)
    // This ensures every non-collection mod gets disabled, not just object conflicts.
    let currently_enabled = collection_repo::get_enabled_mod_id_and_paths(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut currently_enabled_ids = std::collections::HashSet::new();
    for (id, _) in &currently_enabled {
        currently_enabled_ids.insert(id.clone());
    }

    let mut target_ids_set = std::collections::HashSet::new();
    for id in &target_ids {
        target_ids_set.insert(id.clone());
    }

    let mut diff_ids: Vec<String> = Vec::new();

    // mods_to_disable: currently enabled, NOT in targets
    for id in &currently_enabled_ids {
        if !target_ids_set.contains(id) {
            diff_ids.push(id.clone());
        }
    }

    // mods_to_enable: in targets, NOT currently enabled
    for id in &target_ids_set {
        if !currently_enabled_ids.contains(id) {
            diff_ids.push(id.clone());
        }
    }

    let states = if diff_ids.is_empty() {
        Vec::new()
    } else {
        collection_repo::get_mod_states_by_ids(pool, game_id, &diff_ids)
            .await
            .map_err(|e| e.to_string())?
    };

    snapshot_current_state(pool, game_id, safe_mode_enabled).await?;

    let mut result = apply_state_change(pool, watcher_state, states, &target_ids).await?;
    result.warnings.extend(reconcile_warnings);

    // ── Nested mods: toggle via filesystem rename ────────────────────────────
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;

    if let Some(ref mp) = mods_path {
        let nested_changes = apply_nested_mods(watcher_state, mp, &nested_target_paths)?;
        result.changed_count += nested_changes;
    }

    Ok(result)
}

async fn snapshot_current_state(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Delete existing snapshot collection mapping and the collection itself for this game
    collection_repo::delete_snapshot_collection(&mut tx, game_id, safe_mode_enabled)
        .await
        .map_err(|e| e.to_string())?;

    let currently_enabled = collection_repo::get_enabled_mod_ids(&mut tx, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let paths = collection_repo::get_mod_paths_for_ids(&mut tx, &currently_enabled)
        .await
        .unwrap_or_default();

    let snapshot_id = Uuid::new_v4().to_string();
    let name = format!("Unsaved {}", chrono::Local::now().format("%Y%m%d%H%M"));

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

/// Apply nested mod state changes via filesystem rename.
///
/// Walks all nested mods under `mods_path`, enables those whose paths are in
/// `target_paths`, and disables all other currently enabled nested mods.
fn apply_nested_mods(
    watcher_state: &WatcherState,
    mods_path: &str,
    target_paths: &[String],
) -> Result<usize, String> {
    let all_nested = nested_walker::walk_nested_mods(mods_path)?;

    if all_nested.is_empty() && target_paths.is_empty() {
        return Ok(0);
    }

    let _guard = SuppressionGuard::new(&watcher_state.suppressor);
    let mut changed = 0;

    // Helper to get disabled-prefix-free path for comparison
    fn get_canonical_path(path: &str) -> String {
        match rename_for_status(path, true) {
            Ok(Some(p)) => p,
            _ => path.to_string(),
        }
    }

    // Build a set of canonical target paths for O(1) lookup.
    let target_set: std::collections::HashSet<String> =
        target_paths.iter().map(|p| get_canonical_path(p)).collect();

    for nm in &all_nested {
        let canonical_current = get_canonical_path(&nm.folder_path);
        let should_enable = target_set.contains(&canonical_current);

        if should_enable && !nm.is_enabled {
            // Enable: rename DISABLED → enabled
            if let Ok(Some(new_path)) = rename_for_status(&nm.folder_path, true) {
                if Path::new(&nm.folder_path).exists() {
                    fs::rename(&nm.folder_path, &new_path)
                        .map_err(|e| format!("Failed to enable nested mod: {e}"))?;
                    changed += 1;
                }
            }
        } else if !should_enable && nm.is_enabled {
            // Disable: rename enabled → DISABLED
            if let Ok(Some(new_path)) = rename_for_status(&nm.folder_path, false) {
                if Path::new(&nm.folder_path).exists() {
                    fs::rename(&nm.folder_path, &new_path)
                        .map_err(|e| format!("Failed to disable nested mod: {e}"))?;
                    changed += 1;
                }
            }
        }
    }

    Ok(changed)
}
