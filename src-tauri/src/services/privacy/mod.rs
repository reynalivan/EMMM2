use crate::database::{collection_repo, game_repo, mod_repo};
use crate::services::collections;
use crate::services::collections::types::CollectionPreviewMod;
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

#[derive(Debug, Clone, Serialize)]
pub struct CorridorPreview {
    pub leaving_mods: Vec<CollectionPreviewMod>,
    pub target_mods: Vec<CollectionPreviewMod>,
    pub target_description: String,
}

/// Fetches both leaving and target corridor mod lists in a single call.
/// Used by the ModeSwitchConfirmModal to show a side-by-side preview.
pub async fn preview_corridor_switch(
    pool: &SqlitePool,
    game_id: &str,
    current_safe: bool,
    target_safe: bool,
) -> Result<CorridorPreview, String> {
    // Fetch both lists in parallel
    let (leaving_result, target_result) = tokio::join!(
        collections::get_active_mods_preview(pool, game_id, current_safe),
        collections::get_system_disabled_preview(pool, game_id, target_safe),
    );

    let leaving_mods = leaving_result?;
    let target_mods = target_result?;

    let target_description = if target_mods.is_empty() {
        "Empty State (All Disabled)".to_string()
    } else {
        format!("Restoring {} Mods", target_mods.len())
    };

    Ok(CorridorPreview {
        leaving_mods,
        target_mods,
        target_description,
    })
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
    let _leaving_safe = matches!(target_mode, Mode::NSFW); // switching TO nsfw = leaving SFW
    let target_safe = matches!(target_mode, Mode::SFW);

    // Fetch mods_path once for both steps
    let game_paths = game_repo::get_all_game_mod_paths(pool)
        .await
        .map_err(|e| e.to_string())?;
    let mods_path = game_paths.get(game_id).cloned().unwrap_or_default();

    // ─── STEP 1: Disable ALL enabled mods indiscriminately ──────────
    // Guarantee 100% clean-slate before target corridor restoration.
    // This marks them with disabled_reason = 'SYSTEM'
    let disabled_count = disable_all_enabled_mods(pool, watcher_state, game_id, &mods_path).await?;

    // ─── STEP 2: Restore target corridor from its saved collection ──
    let (restored_count, warnings) =
        restore_system_disabled_mods(pool, watcher_state, game_id, target_safe, &mods_path).await?;

    Ok(CorridorSwitchResult {
        disabled_count,
        restored_count,
        warnings,
    })
}

/// Disables ALL ENABLED mods for a game unconditionally.
/// This enforces a 100% clean-slate before restoring the target corridor,
/// preventing rogue background mods from bleeding through.
async fn disable_all_enabled_mods(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    mods_path: &str,
) -> Result<usize, String> {
    let enabled_mods: Vec<_> = collection_repo::get_enabled_mod_id_and_paths(pool, game_id)
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
        Some("SYSTEM"),
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
