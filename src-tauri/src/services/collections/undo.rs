use super::apply::apply_state_change;
use super::types::{ApplyCollectionResult, ModState};
use crate::services::scanner::watcher::WatcherState;
use sqlx::SqlitePool;

pub async fn undo_collection(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<ApplyCollectionResult, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Find the snapshot collection
    let (collection_id, is_safe_context) =
        crate::database::collection_repo::get_last_unsaved_collection_id_and_safe_context(
            &mut *tx, game_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No recent action to undo")?;

    if safe_mode_enabled && !is_safe_context {
        return Err("Snapshot contains non-safe context. Disable Safe Mode to undo.".to_string());
    }

    // Get the target IDs from the snapshot
    let snapshot_mod_ids =
        crate::database::collection_repo::get_collection_item_mod_ids(&mut *tx, &collection_id)
            .await
            .map_err(|e| e.to_string())?;

    // Commit transaction because `apply_state_change` uses its own transaction
    tx.commit().await.map_err(|e| e.to_string())?;

    // Get all currently enabled mods for this game
    let currently_enabled =
        crate::database::collection_repo::get_enabled_mod_id_and_paths(pool, game_id)
            .await
            .map_err(|e| e.to_string())?;

    // We need to disable all currently enabled mods that are NOT in the snapshot
    // And enable all snapshot mods that are currently disabled.
    // So we fetch the status of ALL mods involved (currently enabled + snapshot target)

    // Combine IDs (ensuring uniqueness)
    let mut all_involved_ids = std::collections::HashSet::new();
    for (id, _) in &currently_enabled {
        all_involved_ids.insert(id.clone());
    }
    for id in &snapshot_mod_ids {
        all_involved_ids.insert(id.clone());
    }

    let all_involved_ids_vec: Vec<String> = all_involved_ids.into_iter().collect();

    if all_involved_ids_vec.is_empty() {
        // Nothing to do if both currently enabled and snapshot are completely empty
        delete_snapshot(pool, &collection_id).await?;
        return Ok(ApplyCollectionResult {
            changed_count: 0,
            warnings: vec![],
        });
    }

    let states: Vec<ModState> = crate::database::collection_repo::get_mod_states_by_ids(
        pool,
        game_id,
        &all_involved_ids_vec,
    )
    .await
    .map_err(|e| e.to_string())?;

    let result = apply_state_change(pool, watcher_state, states, &snapshot_mod_ids).await?;

    // Cleanup snapshot after successful undo
    delete_snapshot(pool, &collection_id).await?;

    Ok(result)
}

async fn delete_snapshot(pool: &SqlitePool, collection_id: &str) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    crate::database::collection_repo::delete_collection_items(&mut *tx, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    crate::database::collection_repo::delete_collection_by_id(&mut *tx, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}
