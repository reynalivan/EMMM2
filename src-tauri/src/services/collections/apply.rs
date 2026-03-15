use super::nested_walker;
use super::types::{ApplyCollectionResult, ModState};
use crate::database::collection_repo;
use crate::database::game_repo;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use sqlx::SqlitePool;
use std::collections::HashMap;
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

    if !orphaned.is_empty() {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        let paths_to_lookup: Vec<String> = orphaned.iter().filter_map(|(_, p)| p.clone()).collect();

        let found_map = collection_repo::batch_get_mod_id_by_paths(&mut tx, game_id, &paths_to_lookup)
            .await
            .map_err(|e| e.to_string())?;

        let mut batch_updates = Vec::new();

        for (old_id, maybe_path) in &orphaned {
            if let Some(path) = maybe_path {
                if let Some(new_id) = found_map.get(path) {
                    batch_updates.push((old_id.clone(), new_id.clone()));
                    reconciled_ids.push(new_id.clone());
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

        if !batch_updates.is_empty() {
            collection_repo::batch_update_collection_item_mod_id(&mut tx, collection_id, &batch_updates)
                .await
                .map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
    }

    let target_ids = reconciled_ids;

    // An empty collection is a valid state (Disable All Mods).
    // We let it proceed with an empty target_ids set to clear the loadout.

    // Gather ALL enabled mods + collection targets (same pattern as undo_collection)
    // This ensures every non-collection mod gets disabled, not just object conflicts.
    let currently_enabled = collection_repo::get_enabled_mod_id_and_paths_for_corridor(
        pool,
        game_id,
        safe_mode_enabled,
    )
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
        let nested_changes = apply_nested_mods(watcher_state, mp, &nested_target_paths).await?;
        result.changed_count += nested_changes;
    }

    Ok(result)
}

pub async fn snapshot_current_state(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<(), String> {
    // Walk nested mods OUTSIDE the SQL transaction to prevent connection starvation
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .unwrap_or_default();

    let mut nested_mods = Vec::new();
    if let Some(ref mp) = mods_path {
        let mp_clone = mp.clone();
        nested_mods = tokio::task::spawn_blocking(move || {
            super::nested_walker::walk_nested_mods(&mp_clone).unwrap_or_default()
        })
        .await
        .unwrap_or_default();
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Delete existing snapshot collection mapping and the collection itself for this game
    collection_repo::delete_snapshot_collection(&mut tx, game_id, safe_mode_enabled)
        .await
        .map_err(|e| e.to_string())?;

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

    // O(1) query to bulk-insert all enabled corridor mods straight from the DB
    collection_repo::insert_snapshot_collection_from_state(
        &mut tx,
        &snapshot_id,
        game_id,
        safe_mode_enabled,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Bulk insert filtered nested mods
    let nested_paths: Vec<String> = nested_mods
        .into_iter()
        .filter(|n| n.is_enabled && n.is_safe == safe_mode_enabled)
        .map(|n| n.folder_path)
        .collect();

    collection_repo::batch_insert_nested_collection_items(&mut tx, &snapshot_id, &nested_paths)
        .await
        .map_err(|e| e.to_string())?;

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
    let mut updates: Vec<(String, String, String, Option<String>)> = Vec::new();
    let mut warnings = Vec::new();

    let mut tasks = Vec::new();
    for state in states {
        let next_status = desired
            .get(&state.id)
            .cloned()
            .unwrap_or_else(|| state.status.clone());
        if state.status == next_status {
            continue;
        }

        let new_path_res = rename_for_status(&state.folder_path, next_status == "ENABLED");
        let reason = if next_status == "ENABLED" {
            None
        } else {
            Some("COLLECTION".to_string())
        };

        tasks.push((state, next_status, reason, new_path_res));
    }

    let mut set = tokio::task::JoinSet::new();

    {
        let _guard = SuppressionGuard::new(&watcher_state.suppressor);

        for (state, next_status, reason, new_path_res) in tasks {
            set.spawn_blocking(move || match new_path_res {
                Ok(Some(path)) => {
                    if !Path::new(&state.folder_path).exists() {
                        let folder_name = Path::new(&state.folder_path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| state.id.clone());
                        return Err(format!("Skipping missing mod: {}", folder_name));
                    }

                    match crate::services::fs_utils::file_utils::rename_cross_drive_fallback(
                        Path::new(&state.folder_path),
                        Path::new(&path),
                    ) {
                        Ok(()) => Ok(Some((state.id, next_status, path, reason))),
                        Err(e) => {
                            let folder_name = Path::new(&state.folder_path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| state.id.clone());
                            Err(format!("Failed to rename {}: {}", folder_name, e))
                        }
                    }
                }
                Ok(None) => Ok(Some((state.id, next_status, state.folder_path, reason))),
                Err(e) => Err(e),
            });
        }

        while let Some(res) = set.join_next().await {
            match res {
                Ok(inner_res) => match inner_res {
                    Ok(Some(update)) => updates.push(update),
                    Ok(None) => {}
                    Err(w) => warnings.push(w),
                },
                Err(e) => warnings.push(format!("Join error during collection apply: {}", e)),
            }
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

/// Apply nested mod state changes via filesystem rename asynchronously.
pub async fn apply_nested_mods(
    watcher_state: &WatcherState,
    mods_path: &str,
    target_paths: &[String],
) -> Result<usize, String> {
    let mp_clone = mods_path.to_string();
    let all_nested =
        tokio::task::spawn_blocking(move || nested_walker::walk_nested_mods(&mp_clone))
            .await
            .map_err(|e| format!("Join error walking nested: {}", e))??;

    if all_nested.is_empty() && target_paths.is_empty() {
        return Ok(0);
    }

    fn get_canonical_path(path: &str) -> String {
        match rename_for_status(path, true) {
            Ok(Some(p)) => p,
            _ => path.to_string(),
        }
    }

    let target_set: std::collections::HashSet<String> =
        target_paths.iter().map(|p| get_canonical_path(p)).collect();

    let mut set: tokio::task::JoinSet<Result<usize, String>> = tokio::task::JoinSet::new();
    let mut changed = 0;

    {
        let _guard = SuppressionGuard::new(&watcher_state.suppressor);

        for nm in all_nested {
            let canonical_current = get_canonical_path(&nm.folder_path);
            let should_enable = target_set.contains(&canonical_current);

            if should_enable && !nm.is_enabled {
                set.spawn_blocking(move || {
                    if let Ok(Some(new_path)) = rename_for_status(&nm.folder_path, true) {
                        if Path::new(&nm.folder_path).exists() {
                            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(
                                Path::new(&nm.folder_path),
                                Path::new(&new_path),
                            )
                            .map_err(|e| format!("Failed to enable nested mod: {}", e))?;
                            return Ok(1);
                        }
                    }
                    Ok(0)
                });
            } else if !should_enable && nm.is_enabled {
                set.spawn_blocking(move || {
                    if let Ok(Some(new_path)) = rename_for_status(&nm.folder_path, false) {
                        if Path::new(&nm.folder_path).exists() {
                            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(
                                Path::new(&nm.folder_path),
                                Path::new(&new_path),
                            )
                            .map_err(|e| format!("Failed to disable nested mod: {}", e))?;
                            return Ok(1);
                        }
                    }
                    Ok(0)
                });
            }
        }

        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(c)) => changed += c,
                Ok(Err(e)) => log::warn!("{}", e),
                Err(e) => log::warn!("Join error applying nested mod: {}", e),
            }
        }
    }

    Ok(changed)
}
