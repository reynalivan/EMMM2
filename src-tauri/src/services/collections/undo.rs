use super::apply::apply_state_change;
use super::resolve_current_effective_corridor_state;
use super::root_resolution::is_foldergrid_level_mod_path;
use super::types::{ApplyCollectionResult, ModState};
use crate::services::scanner::watcher::WatcherState;
use sqlx::SqlitePool;

pub async fn undo_collection(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<ApplyCollectionResult, String> {
    let undo_collection_id =
        crate::database::corridor_state_repo::get_corridor_state(pool, game_id, safe_mode_enabled)
            .await
            .map_err(|e| e.to_string())?
            .undo_collection_id
            .ok_or("No recent action to undo")?;

    let effective_target =
        super::apply::resolve_apply_collection_target(pool, &undo_collection_id, game_id).await?;
    let snapshot_mod_ids = effective_target.effective_db_mod_ids;
    let nested_target_paths = effective_target.effective_nested_paths;
    let disabled_object_ids = effective_target.disabled_object_ids;

    // Get all currently enabled mods for this game
    let currently_enabled =
        resolve_current_effective_corridor_state(pool, game_id, safe_mode_enabled).await?;

    // We need to disable all currently enabled mods that are NOT in the snapshot
    // And enable all snapshot mods that are currently disabled.
    // So we fetch the status of ALL mods involved (currently enabled + snapshot target)

    // Combine IDs (ensuring uniqueness)
    let mut all_involved_ids = std::collections::HashSet::new();
    for id in &currently_enabled.effective_db_mod_ids {
        all_involved_ids.insert(id.clone());
    }
    for id in &snapshot_mod_ids {
        all_involved_ids.insert(id.clone());
    }

    let all_involved_ids_vec: Vec<String> = all_involved_ids.into_iter().collect();

    if all_involved_ids_vec.is_empty() {
        // Nothing to do if both currently enabled and snapshot are completely empty
        delete_snapshot(pool, &undo_collection_id).await?;
        // Clear corridor_state: undo consumed, no active collection.
        if let Err(e) = crate::database::corridor_state_repo::upsert_corridor_state(
            pool,
            game_id,
            safe_mode_enabled,
            None,
            None,
        )
        .await
        {
            log::warn!("Failed to clear corridor_state after undo (empty case): {e}");
        }
        return Ok(ApplyCollectionResult {
            changed_count: 0,
            warnings: vec![],
        });
    }

    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;

    let states: Vec<ModState> = crate::database::collection_repo::get_mod_states_by_ids(
        pool,
        game_id,
        &all_involved_ids_vec,
    )
    .await
    .map_err(|e| e.to_string())?
    .into_iter()
    .filter(|state| is_foldergrid_level_mod_path(&state.folder_path, mods_path.as_deref()))
    .collect();

    let mut result = apply_state_change(
        pool,
        watcher_state,
        game_id,
        states,
        &snapshot_mod_ids,
        &disabled_object_ids,
    )
    .await?;
    result.warnings.extend(effective_target.warnings);

    // ── Nested mods: toggle via filesystem rename ────────────────────────────
    if let Some(ref mp) = mods_path {
        let nested_changes =
            super::apply::apply_nested_mods(watcher_state, game_id, mp, &nested_target_paths)
                .await?;
        result.changed_count += nested_changes;
    }

    // Cleanup snapshot after successful undo
    delete_snapshot(pool, &undo_collection_id).await?;

    // Clear corridor_state: no active collection after undo, and undo is now consumed.
    if let Err(e) = crate::database::corridor_state_repo::upsert_corridor_state(
        pool,
        game_id,
        safe_mode_enabled,
        None,
        None,
    )
    .await
    {
        log::warn!("Failed to clear corridor_state after undo: {e}");
    }

    Ok(result)
}

async fn delete_snapshot(pool: &SqlitePool, collection_id: &str) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    crate::database::collection_repo::delete_collection_by_id(&mut tx, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}
