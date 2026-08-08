//! Public entry points: enqueue, serialize per game, and drain coalesced work.

use crate::domain::errors::AppError;
use crate::services::disk_reconcile::types::DiskReconcileResult;
use crate::services::scanner::watcher::ModWatchEvent;

use super::request::{DiskReconcileContext, DiskReconcileRequest};
use super::run::{run_refresh_once, RefreshRequest};

/// Disk Reconcile watcher batches must stay disk-only.
/// Watcher must never invoke the Deep Match Scanner pipeline.
pub async fn reconcile_disk_state_from_watcher_batch(
    context: DiskReconcileContext<'_>,
    game_id: String,
    changed_paths: Vec<String>,
    watcher_events: &[ModWatchEvent],
) -> Result<DiskReconcileResult, AppError> {
    reconcile_disk_state(
        context,
        DiskReconcileRequest::watcher_batch(game_id, changed_paths, watcher_events),
    )
    .await
}

/// Disk Reconcile keeps runtime projection aligned with filesystem reality.
/// Watcher, focus, and Mods view entry must call this path only.
/// Do not add Deep Match Scanner logic here.
pub async fn reconcile_disk_state(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
) -> Result<DiskReconcileResult, AppError> {
    let game_id = request.game_id;
    let requested_version = context.state.enqueue_request(
        &game_id,
        request.reason.clone(),
        &request.changed_paths,
        request.force_full,
        &request.watcher_events,
    );
    let game_lock = context.state.lock_for_game(&game_id);
    let _guard = game_lock.lock().await;

    loop {
        let Some(pending) = context
            .state
            .take_pending_or_cached(&game_id, requested_version)?
        else {
            return context.state.last_result(&game_id);
        };

        let result = match run_refresh_once(RefreshRequest {
            context: context.clone(),
            game_id: &game_id,
            reason: pending.reason.clone(),
            changed_paths: pending.changed_paths.iter().cloned().collect(),
            force_full: pending.force_full,
            watcher_events: pending.watcher_events.clone(),
        })
        .await
        {
            Ok(result) => result,
            Err(error) => {
                context.state.requeue_pending(&game_id, pending);
                return Err(error);
            }
        };

        let has_pending = context
            .state
            .finish_run(&game_id, pending.max_version, &result);
        if !has_pending {
            return Ok(result);
        }
    }
}
