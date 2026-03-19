use super::apply_progress;
use super::nested_walker;
use super::resolve_current_effective_corridor_state;
use super::root_resolution::{
    display_name_for_path, is_effectively_enabled_folder_path, is_foldergrid_level_mod_path,
    resolve_existing_preview_path,
};
use super::types::{
    ApplyCollectionProgressPhase, ApplyCollectionResult, CollectionObjectState, ModState,
};
use crate::database::collection_repo;
use crate::services::corridor_constants::DISABLED_REASON_COLLECTION;
use crate::services::mods::core_ops::standardize_prefix;
use crate::services::path_key::{
    canonical_collection_path_key, canonical_path_key_for_path, resolve_collection_path,
};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use uuid::Uuid;

static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").expect("valid regex"));

pub(crate) struct ResolvedCollectionApplyTarget {
    pub effective_db_mod_ids: Vec<String>,
    pub effective_nested_paths: Vec<String>,
    pub object_states: Vec<CollectionObjectState>,
    pub disabled_object_ids: HashSet<String>,
    pub warnings: Vec<String>,
}

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
            "Collection contains non-safe mods. Disable Privacy Mode to apply.".to_string(),
        );
    }

    if !safe_mode_enabled && is_safe_context {
        return Err("Collection is Safe. Enable Privacy Mode to apply.".to_string());
    }

    apply_progress::start_apply_progress(game_id, &collection_name, safe_mode_enabled);

    let result = apply_collection_inner(
        pool,
        watcher_state,
        collection_id,
        game_id,
        safe_mode_enabled,
    )
    .await;

    match &result {
        Ok(_) => apply_progress::finish_apply_progress(game_id),
        Err(error) => apply_progress::fail_apply_progress(game_id, error),
    }

    result
}

async fn apply_collection_inner(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    collection_id: &str,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<ApplyCollectionResult, String> {
    let effective_target = resolve_apply_collection_target(pool, collection_id, game_id).await?;

    let target_ids = effective_target.effective_db_mod_ids;
    let nested_target_paths = effective_target.effective_nested_paths;
    let target_object_states = effective_target.object_states;
    let disabled_object_ids = effective_target.disabled_object_ids;

    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let mods_root = mods_path
        .clone()
        .ok_or_else(|| "Failed to get game mods path".to_string())?;

    let snapshot_id = snapshot_current_state(pool, game_id, safe_mode_enabled).await?;

    let object_changes = plan_object_state_changes(pool, game_id, &target_object_states).await?;
    apply_progress::add_apply_progress_total(game_id, object_changes.len());
    let object_change_count =
        apply_object_state_changes(pool, watcher_state, game_id, &mods_root, &object_changes)
            .await?;

    let current_state =
        resolve_current_effective_corridor_state(pool, game_id, safe_mode_enabled).await?;
    let currently_enabled_ids: std::collections::HashSet<String> =
        current_state.effective_db_mod_ids.into_iter().collect();
    let target_ids_set: std::collections::HashSet<String> = target_ids.iter().cloned().collect();

    let mut diff_ids: Vec<String> = currently_enabled_ids
        .difference(&target_ids_set)
        .cloned()
        .collect();

    for id in target_ids_set.difference(&currently_enabled_ids) {
        diff_ids.push(id.clone());
    }

    if !disabled_object_ids.is_empty() {
        let disabled_object_vec: Vec<String> = disabled_object_ids.iter().cloned().collect();
        let force_disable_ids = collection_repo::get_enabled_mod_ids_for_object_ids(
            pool,
            game_id,
            safe_mode_enabled,
            &disabled_object_vec,
        )
        .await
        .map_err(|e| e.to_string())?;

        for mod_id in force_disable_ids {
            if !diff_ids.contains(&mod_id) {
                diff_ids.push(mod_id);
            }
        }
    }

    let states = if diff_ids.is_empty() {
        Vec::new()
    } else {
        collection_repo::get_mod_states_by_ids(pool, game_id, &diff_ids)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|state| is_foldergrid_level_mod_path(&state.folder_path, mods_path.as_deref()))
            .collect()
    };

    apply_progress::add_apply_progress_total(game_id, states.len());
    let mut result = apply_state_change(
        pool,
        watcher_state,
        game_id,
        states,
        &target_ids,
        &disabled_object_ids,
    )
    .await?;
    result.changed_count += object_change_count;
    result.warnings.extend(effective_target.warnings);

    let nested_changes =
        apply_nested_mods(watcher_state, game_id, &mods_root, &nested_target_paths).await?;
    result.changed_count += nested_changes;

    if let Err(e) = crate::database::corridor_state_repo::upsert_corridor_state(
        pool,
        game_id,
        safe_mode_enabled,
        Some(collection_id),
        Some(&snapshot_id),
    )
    .await
    {
        log::warn!("Failed to update corridor_state after apply: {e}");
    }

    Ok(result)
}
/// Snapshots the CURRENT enabled state of a corridor into an `is_last_unsaved` collection.
/// Returns the new snapshot's UUID, allowing callers to store it as an undo pointer.
/// This function is called BEFORE any state changes are applied so the snapshot
/// represents the pre-change state (enabling Undo).
pub async fn snapshot_current_state(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<String, String> {
    let current_runtime_snapshot =
        super::resolve_corridor_runtime_snapshot(pool, game_id, safe_mode_enabled).await?;
    let current_state =
        resolve_current_effective_corridor_state(pool, game_id, safe_mode_enabled).await?;

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
    collection_repo::batch_insert_collection_object_states(
        &mut tx,
        &snapshot_id,
        &current_state.object_states,
    )
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    super::persist_collection_runtime_materialization(
        pool,
        &snapshot_id,
        &current_runtime_snapshot.roots,
        &current_runtime_snapshot.object_states,
        mods_path.as_deref(),
    )
    .await?;
    Ok(snapshot_id)
}

pub(crate) async fn resolve_apply_collection_target(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<ResolvedCollectionApplyTarget, String> {
    let preview = super::get_collection_runtime_preview(pool, collection_id, game_id).await?;
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let object_states: Vec<CollectionObjectState> = preview
        .object_states
        .iter()
        .map(|state| CollectionObjectState {
            object_id: state.object_id.clone(),
            is_enabled: state.is_enabled,
        })
        .collect();
    let disabled_object_ids: HashSet<String> = object_states
        .iter()
        .filter(|state| !state.is_enabled)
        .map(|state| state.object_id.clone())
        .collect();
    let candidate_paths: Vec<String> = preview
        .roots
        .iter()
        .map(|root| root.folder_path.clone())
        .collect();
    let resolved_map = collection_repo::batch_get_mod_id_by_paths_pool(
        pool,
        game_id,
        &candidate_paths,
        mods_path.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let mut resolved_ids: Vec<String> = resolved_map.into_values().collect();
    resolved_ids.sort();
    resolved_ids.dedup();

    let effective_db_mod_ids = if resolved_ids.is_empty() {
        Vec::new()
    } else {
        collection_repo::get_mod_states_by_ids(pool, game_id, &resolved_ids)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|state| {
                is_collection_db_mod_target(state, &disabled_object_ids, mods_path.as_deref())
            })
            .map(|state| state.id)
            .collect()
    };
    let effective_db_paths =
        collection_repo::get_mod_paths_for_ids_pool(pool, &effective_db_mod_ids)
            .await
            .map_err(|e| e.to_string())?;
    let db_path_keys: HashSet<String> = effective_db_paths
        .values()
        .filter_map(|path| canonical_collection_path_key(path, mods_path.as_deref()))
        .collect();

    let mut warnings = Vec::new();
    let effective_nested_paths = normalize_preview_target_paths(
        candidate_paths,
        mods_path.as_deref(),
        &db_path_keys,
        &mut warnings,
    );

    Ok(ResolvedCollectionApplyTarget {
        effective_db_mod_ids,
        effective_nested_paths,
        object_states,
        disabled_object_ids,
        warnings,
    })
}

#[derive(Clone)]
struct ObjectStateChange {
    object_id: String,
    old_path: String,
    new_path: String,
    enable: bool,
}

async fn plan_object_state_changes(
    pool: &SqlitePool,
    game_id: &str,
    target_object_states: &[CollectionObjectState],
) -> Result<Vec<ObjectStateChange>, String> {
    let current_states = collection_repo::get_current_object_states_for_game(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let current_by_id: HashMap<String, bool> = current_states
        .into_iter()
        .map(|state| (state.object_id, state.is_enabled))
        .collect();

    let mut changes = Vec::new();
    for target_state in target_object_states {
        let Some(current_enabled) = current_by_id.get(&target_state.object_id).copied() else {
            continue;
        };
        if current_enabled == target_state.is_enabled {
            continue;
        }

        let Some(old_path) =
            crate::database::object_repo::get_folder_path(pool, &target_state.object_id)
                .await
                .map_err(|e| e.to_string())?
        else {
            continue;
        };
        if old_path.trim().is_empty() {
            continue;
        }

        let new_path = standardize_prefix(&old_path, target_state.is_enabled);
        if new_path == old_path {
            continue;
        }

        changes.push(ObjectStateChange {
            object_id: target_state.object_id.clone(),
            old_path,
            new_path,
            enable: target_state.is_enabled,
        });
    }

    Ok(changes)
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

fn normalize_preview_target_paths(
    paths: Vec<String>,
    mods_path: Option<&str>,
    db_path_keys: &HashSet<String>,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for original_path in paths {
        if !is_effectively_enabled_folder_path(&original_path, mods_path) {
            continue;
        }

        let resolved_path = resolve_existing_preview_path(&original_path, mods_path)
            .or_else(|| resolve_preview_path(&original_path, mods_path))
            .unwrap_or_else(|| PathBuf::from(&original_path));
        if !resolved_path.exists() {
            warnings.push(format!(
                "Missing mod: {}",
                display_name_for_path(&original_path)
            ));
            continue;
        }

        let resolved_string = resolved_path.to_string_lossy().to_string();
        let path_key = canonical_collection_path_key(&resolved_string, mods_path)
            .unwrap_or_else(|| canonical_path_key_for_path(&resolved_path));
        if db_path_keys.contains(&path_key) {
            continue;
        }
        if seen.insert(path_key) {
            normalized.push(resolved_string);
        }
    }

    normalized
}

fn resolve_preview_path(folder_path: &str, mods_path: Option<&str>) -> Option<PathBuf> {
    let resolved = resolve_collection_path(folder_path, mods_path)?;
    Some(resolved.components().collect())
}

async fn apply_object_state_changes(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    mods_root: &str,
    changes: &[ObjectStateChange],
) -> Result<usize, String> {
    if changes.is_empty() {
        return Ok(0);
    }

    apply_progress::set_apply_progress_phase(game_id, ApplyCollectionProgressPhase::Renaming, None);

    let mut changed = 0;
    {
        let _guard = SuppressionGuard::new(&watcher_state.suppressor);
        for change in changes {
            let source = Path::new(mods_root).join(&change.old_path);
            let target = Path::new(mods_root).join(&change.new_path);
            if !source.exists() {
                return Err(format!(
                    "Object folder does not exist: {}",
                    source.display()
                ));
            }
            if target.exists() {
                return Err(format!(
                    "Target object path already exists: {}",
                    target.display()
                ));
            }

            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(&source, &target)
                .map_err(|e| {
                    format!("Failed to rename object folder '{}': {e}", change.old_path)
                })?;

            let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
            sqlx::query(
                "UPDATE objects
                 SET folder_path = ?,
                     folder_path_key = ?,
                     name = CASE WHEN name = ? THEN ? ELSE name END,
                     name_key = CASE WHEN name = ? THEN ? ELSE name_key END
                 WHERE id = ? AND game_id = ?",
            )
            .bind(&change.new_path)
            .bind(crate::services::path_key::folder_path_key(
                &change.new_path,
                None,
            ))
            .bind(&change.old_path)
            .bind(&change.new_path)
            .bind(&change.old_path)
            .bind(crate::services::path_key::object_name_key(&change.new_path))
            .bind(&change.object_id)
            .bind(game_id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| e.to_string())?;

            for (old_prefix, new_prefix) in [
                (
                    format!("{}\\", change.old_path),
                    format!("{}\\", change.new_path),
                ),
                (
                    format!("{}/", change.old_path),
                    format!("{}/", change.new_path),
                ),
            ] {
                crate::database::mod_repo::update_child_paths_tx(
                    &mut tx,
                    game_id,
                    &old_prefix,
                    &new_prefix,
                    Some(mods_root),
                )
                .await
                .map_err(|e| e.to_string())?;
            }

            crate::database::mod_repo::update_status_and_reason_for_object(
                &mut tx,
                game_id,
                &change.new_path,
                if change.enable { "ENABLED" } else { "DISABLED" },
                if change.enable {
                    None
                } else {
                    Some(DISABLED_REASON_COLLECTION)
                },
            )
            .await
            .map_err(|e| e.to_string())?;

            tx.commit().await.map_err(|e| e.to_string())?;
            changed += 1;
            let object_label =
                crate::database::object_repo::get_object_name_by_id(pool, &change.object_id)
                    .await
                    .unwrap_or(None)
                    .unwrap_or_else(|| change.new_path.clone());
            apply_progress::advance_apply_progress(game_id, Some(object_label));
        }
    }

    Ok(changed)
}

pub async fn apply_state_change(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    states: Vec<ModState>,
    target_ids: &[String],
    disabled_object_ids: &HashSet<String>,
) -> Result<ApplyCollectionResult, String> {
    let desired: HashMap<String, String> = states
        .iter()
        .map(|state| {
            let disabled_by_object_state = state
                .object_id
                .as_ref()
                .is_some_and(|object_id| disabled_object_ids.contains(object_id));

            let target = if !disabled_by_object_state && target_ids.contains(&state.id) {
                "ENABLED"
            } else {
                "DISABLED"
            };
            (state.id.clone(), target.to_string())
        })
        .collect();

    let (changed, warnings) =
        apply_with_desired_status(pool, watcher_state, game_id, states, desired).await?;

    Ok(ApplyCollectionResult {
        changed_count: changed,
        warnings,
    })
}

async fn apply_with_desired_status(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
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
            Some(DISABLED_REASON_COLLECTION.to_string())
        };

        tasks.push((state, next_status, reason, new_path_res));
    }

    if tasks.is_empty() {
        return Ok((0, warnings));
    }

    let mut set = tokio::task::JoinSet::new();
    apply_progress::set_apply_progress_phase(game_id, ApplyCollectionProgressPhase::Renaming, None);

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
                    Ok(Some(update)) => {
                        let item_label = Path::new(&update.2)
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| update.0.clone());
                        updates.push(update);
                        apply_progress::advance_apply_progress(game_id, Some(item_label));
                    }
                    Ok(None) => {}
                    Err(w) => warnings.push(w),
                },
                Err(e) => warnings.push(format!("Join error during collection apply: {}", e)),
            }
        }
    }

    apply_progress::set_apply_progress_phase(
        game_id,
        ApplyCollectionProgressPhase::UpdatingDb,
        None,
    );
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    collection_repo::batch_update_mods_status_and_path(pool, mods_path.as_deref(), &updates)
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
    game_id: &str,
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

    let target_set: std::collections::HashSet<String> = target_paths
        .iter()
        .filter_map(|path| enabled_nested_variant_key(path, mods_path))
        .collect();
    let total_changes = count_nested_changes_from_walk(&all_nested, mods_path, &target_set);
    if total_changes == 0 {
        return Ok(0);
    }
    apply_progress::add_apply_progress_total(game_id, total_changes);

    let mut set: tokio::task::JoinSet<Result<usize, String>> = tokio::task::JoinSet::new();
    let mut changed = 0;
    apply_progress::set_apply_progress_phase(game_id, ApplyCollectionProgressPhase::Renaming, None);

    {
        let _guard = SuppressionGuard::new(&watcher_state.suppressor);

        for nm in all_nested {
            let Some(canonical_current_key) =
                enabled_nested_variant_key(&nm.folder_path, mods_path)
            else {
                continue;
            };
            let should_enable = target_set.contains(&canonical_current_key);

            if should_enable && !nm.is_enabled {
                let mods_root = mods_path.to_string();
                let folder_path = nm.folder_path.clone();
                set.spawn_blocking(move || {
                    if Path::new(&folder_path).exists() {
                        rename_nested_mod_chain_to_enabled(&mods_root, &folder_path)?;
                        return Ok(1);
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
                Ok(Ok(c)) => {
                    changed += c;
                    if c > 0 {
                        apply_progress::advance_apply_progress(game_id, None);
                    }
                }
                Ok(Err(e)) => log::warn!("{}", e),
                Err(e) => log::warn!("Join error applying nested mod: {}", e),
            }
        }
    }

    Ok(changed)
}
fn count_nested_changes_from_walk(
    nested_mods: &[nested_walker::NestedModState],
    mods_path: &str,
    target_set: &HashSet<String>,
) -> usize {
    nested_mods
        .iter()
        .filter(|nm| {
            let Some(canonical_current_key) =
                enabled_nested_variant_key(&nm.folder_path, mods_path)
            else {
                return false;
            };
            let should_enable = target_set.contains(&canonical_current_key);
            (should_enable && !nm.is_enabled) || (!should_enable && nm.is_enabled)
        })
        .count()
}

fn enabled_nested_variant_key(path: &str, mods_path: &str) -> Option<String> {
    let enabled_path = enabled_nested_variant_path(path, mods_path)?;
    canonical_collection_path_key(&enabled_path, Some(mods_path))
}

fn enabled_nested_variant_path(path: &str, mods_path: &str) -> Option<String> {
    let mods_root = Path::new(mods_path);
    let resolved = resolve_collection_path(path, Some(mods_path))?;
    let relative = resolved.strip_prefix(mods_root).ok()?;

    let mut normalized = mods_root.to_path_buf();
    for component in relative.components() {
        let display_segment = component.as_os_str().to_string_lossy().to_string();
        normalized.push(standardize_prefix(&display_segment, true));
    }

    Some(normalized.to_string_lossy().to_string())
}

fn rename_nested_mod_chain_to_enabled(mods_path: &str, folder_path: &str) -> Result<(), String> {
    let mods_root = Path::new(mods_path);
    let resolved = resolve_collection_path(folder_path, Some(mods_path))
        .ok_or_else(|| format!("Invalid nested mod path: {folder_path}"))?;
    let relative = resolved
        .strip_prefix(mods_root)
        .map_err(|_| format!("Nested mod path is outside mods root: {folder_path}"))?;

    let mut current_parent = mods_root.to_path_buf();
    for component in relative.components() {
        let current_segment = component.as_os_str().to_string_lossy().to_string();
        let next_segment = standardize_prefix(&current_segment, true);
        let current_path = current_parent.join(&current_segment);
        let next_path = current_parent.join(&next_segment);

        if current_segment != next_segment {
            if next_path.exists() && current_path != next_path {
                return Err(format!(
                    "Target path already exists: {}",
                    next_path.display()
                ));
            }

            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(
                &current_path,
                &next_path,
            )
            .map_err(|e| format!("Failed to enable nested mod chain: {e}"))?;
            current_parent = next_path;
            continue;
        }

        current_parent = current_path;
    }

    Ok(())
}
