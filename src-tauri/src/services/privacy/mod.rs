use crate::database::{collection_repo, game_repo, mod_repo};
use crate::services::collections;
use crate::services::corridor_constants::DISABLED_REASON_SYSTEM;
use crate::services::scanner::watcher::WatcherState;
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug)]
pub enum Mode {
    SFW,
    NSFW,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorridorSwitchResult {
    pub disabled_count: usize,
    pub restored_count: usize,
    pub warnings: Vec<String>,
}

/// Executes the atomic Corridor Handoff between SFW and NSFW modes.
///
/// 1. Snapshot leaving corridor's enabled mods as an `is_last_unsaved` collection
/// 2. Disable ALL leaving corridor mods (DB + disk rename)
/// 3. Lookup target corridor's last saved state (is_last_unsaved collection)
/// 4. Restore target corridor by re-enabling those mods
/// 5. Return counts + any missing-mod warnings
pub async fn switch_mode(
    target_mode: Mode,
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
) -> Result<CorridorSwitchResult, String> {
    let leaving_safe = matches!(target_mode, Mode::NSFW); // switching TO nsfw = leaving SFW
    let target_safe = matches!(target_mode, Mode::SFW);

    // Fetch mods_path once for both steps
    let game_paths = game_repo::get_all_game_mod_paths(pool)
        .await
        .map_err(|e| e.to_string())?;
    let mods_path = game_paths.get(game_id).cloned().unwrap_or_default();

    // ─── STEP 0: Snapshot the LEAVING corridor ─────────────────────
    // This runs BEFORE any state changes so the snapshot captures what was enabled.
    // Gives the user the ability to "undo" the mode switch from within the leaving corridor.
    let leaving_snapshot_id = collections::snapshot_current_state(pool, game_id, leaving_safe)
        .await
        .unwrap_or_else(|e| {
            log::warn!("snapshot_current_state failed for leaving corridor: {e}");
            String::new()
        });

    if !leaving_snapshot_id.is_empty() {
        let existing_active_collection_id =
            match crate::database::corridor_state_repo::get_corridor_state(
                pool,
                game_id,
                leaving_safe,
            )
            .await
            {
                Ok(state) => state.active_collection_id,
                Err(e) => {
                    log::warn!("Failed to read corridor_state for leaving corridor: {e}");
                    None
                }
            };

        if let Err(e) = crate::database::corridor_state_repo::upsert_corridor_state(
            pool,
            game_id,
            leaving_safe,
            existing_active_collection_id.as_deref(),
            Some(&leaving_snapshot_id),
        )
        .await
        {
            log::warn!("Failed to update corridor_state for leaving corridor: {e}");
        }
    }

    // ─── STEP 1: Disable enabled mods in the leaving corridor ───────
    // This marks them with disabled_reason = 'SYSTEM'
    let disabled_count =
        disable_enabled_mods_in_corridor(pool, watcher_state, game_id, leaving_safe, &mods_path)
            .await?;

    // ─── STEP 2: Restore target corridor from its saved collection ──
    let (restored_count, warnings) =
        restore_system_disabled_mods(pool, watcher_state, game_id, target_safe, &mods_path).await?;

    Ok(CorridorSwitchResult {
        disabled_count,
        restored_count,
        warnings,
    })
}

/// Disables enabled mods only in the corridor being left.
/// This preserves the target corridor's current state and avoids unnecessary
/// disable-then-restore churn on mods that are already in the destination corridor.
async fn disable_enabled_mods_in_corridor(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    leaving_safe: bool,
    mods_path: &str,
) -> Result<usize, String> {
    let enabled_mods: Vec<_> =
        collection_repo::get_enabled_mod_id_and_paths_for_corridor(pool, game_id, leaving_safe)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|(_, path)| {
                // Filter out depth-1 folders (e.g. "Acheron"). These are Object containers.
                // We only want to disable actual sub-mods (e.g. "Acheron/PinkAcheron").
                std::path::Path::new(path).components().count() > 1
            })
            .collect();

    if enabled_mods.is_empty() {
        return Ok(0);
    }

    if mods_path.is_empty() {
        log::warn!(
            "No mods_path found for game '{}', skipping all mods",
            game_id
        );
        return Ok(0);
    }

    let (disabled_count, warnings) = crate::services::mods::bulk_ops::bulk_toggle_mods(
        pool,
        watcher_state,
        mods_path,
        game_id,
        enabled_mods,
        false,
        Some(DISABLED_REASON_SYSTEM),
    )
    .await?;

    for warning in warnings {
        log::warn!("Soft failure disabling mod: {}", warning);
    }

    Ok(disabled_count)
}

/// Restores mods that were previously disabled by the SYSTEM switch.
/// Returns (restored_count, warnings) where warnings list missing mods or errors.
async fn restore_system_disabled_mods(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    target_safe: bool,
    mods_path: &str,
) -> Result<(usize, Vec<String>), String> {
    // Get target mods that should be restored
    let system_disabled = mod_repo::get_system_disabled_mods(pool, game_id, target_safe)
        .await
        .map_err(|e| e.to_string())?;

    if system_disabled.is_empty() {
        return Ok((0, vec![]));
    }

    if mods_path.is_empty() {
        return Ok((0, vec![]));
    }

    let (restored_count, warnings) = crate::services::mods::bulk_ops::bulk_toggle_mods(
        pool,
        watcher_state,
        mods_path,
        game_id,
        system_disabled,
        true,
        None,
    )
    .await?;

    Ok((restored_count, warnings))
}

#[cfg(test)]
#[path = "tests/privacy_service_tests.rs"]
mod tests;
