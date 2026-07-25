//! One reconcile pass: resolves the game, runs the disk projection, then
//! applies runtime side-effects and builds the result.

use crate::services::disk_reconcile::reconcile::{
    reconcile_disk_projection, ReconcileDiskProjectionRequest, ReconcileOutcome,
};
use crate::services::disk_reconcile::types::{
    DiskReconcileReason, DiskReconcileResult, DiskReconcileStatus,
};
use crate::services::scanner::watcher::ModWatchEvent;

use super::request::DiskReconcileContext;

struct RuntimeEffectsRequest<'a> {
    context: DiskReconcileContext<'a>,
    game_id: &'a str,
    reason: DiskReconcileReason,
    outcome: ReconcileOutcome,
}

async fn finalize_runtime_effects(request: RuntimeEffectsRequest<'_>) -> DiskReconcileResult {
    let collections_changed = request.outcome.status == DiskReconcileStatus::Applied
        && (request.outcome.folders_changed
            || request.outcome.objects_changed
            || request.outcome.runtime_file_changed);

    let overlay_refresh_triggered = if request.outcome.status == DiskReconcileStatus::Applied {
        match crate::services::app::runtime_effects::finalize_runtime_side_effects(
            request.context.pool,
            request.context.config,
            request.context.watcher_suppressor,
            request.game_id,
            &[true, false],
            collections_changed,
            request.outcome.folders_changed || request.outcome.runtime_file_changed,
        )
        .await
        {
            Ok(triggered) => triggered,
            Err(error) => {
                log::warn!(
                    "Disk Reconcile runtime side-effects failed for game '{}': {}",
                    request.game_id,
                    error
                );
                false
            }
        }
    } else {
        false
    };

    DiskReconcileResult {
        game_id: request.game_id.to_string(),
        reason: request.reason,
        status: request.outcome.status,
        error_message: request.outcome.error_message,
        changed_roots: request.outcome.changed_roots,
        objects_changed: request.outcome.objects_changed,
        folders_changed: request.outcome.folders_changed,
        collections_changed,
        runtime_file_changed: request.outcome.runtime_file_changed,
        overlay_refresh_triggered,
        thumbnail_roots: request.outcome.thumbnail_roots,
        cleared_selection_paths: request.outcome.cleared_selection_paths,
        path_updates: request.outcome.path_updates,
        collection_reference_impact: request.outcome.collection_reference_impact,
        change_summary: request.outcome.change_summary,
    }
}

pub(super) struct RefreshRequest<'a> {
    pub(super) context: DiskReconcileContext<'a>,
    pub(super) game_id: &'a str,
    pub(super) reason: DiskReconcileReason,
    pub(super) changed_paths: Vec<String>,
    pub(super) force_full: bool,
    pub(super) watcher_events: Vec<ModWatchEvent>,
}

pub(super) async fn run_refresh_once(
    request: RefreshRequest<'_>,
) -> Result<DiskReconcileResult, String> {
    let settings = request.context.config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == request.game_id)
        .ok_or_else(|| format!("Game '{}' not found for disk reconcile", request.game_id))?;
    let watcher_events = if request.watcher_events.is_empty() {
        None
    } else {
        Some(request.watcher_events.as_slice())
    };

    let reconcile = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: request.context.pool,
        game_id: request.game_id,
        mods_path: &game.mod_path,
        safe_mode_keywords: &settings.safe_mode.keywords,
        reason: &request.reason,
        changed_paths: &request.changed_paths,
        force_full: request.force_full,
        watcher_events,
    })
    .await?;

    Ok(finalize_runtime_effects(RuntimeEffectsRequest {
        context: request.context,
        game_id: request.game_id,
        reason: request.reason,
        outcome: reconcile,
    })
    .await)
}
