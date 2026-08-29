//! Public entry points: serialize per game and run one reconcile request.

use crate::shared::errors::AppError;
use crate::modules::workspace::application::disk_reconcile::types::DiskReconcileResult;
use crate::modules::workspace::application::scanner::watcher::{ModWatchEvent, WatcherSession};

use super::request::{DiskReconcileContext, DiskReconcileRequest};
use super::run::{run_refresh_once, RefreshRequest};

/// Disk Reconcile watcher batches must stay disk-only.
/// Watcher must never invoke the Deep Match Scanner pipeline.
pub async fn reconcile_disk_state_from_watcher_batch(
    context: DiskReconcileContext<'_>,
    game_id: String,
    changed_paths: Vec<String>,
    watcher_events: &[ModWatchEvent],
    watcher_session: WatcherSession,
) -> Result<DiskReconcileResult, AppError> {
    reconcile_disk_state(
        context,
        DiskReconcileRequest::watcher_batch(game_id, changed_paths, watcher_events)
            .for_watcher_session(watcher_session),
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
    let game_lock = context.state.lock_for_game(&request.game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = context.operation_lock.acquire_for_reconcile().await;
    reconcile_disk_state_under_locks(context, request, &game_guard, &operation_guard).await
}

/// Run one reconcile request while the caller keeps both serialization
/// leases. Source-directory activation uses this entrypoint so changing the
/// configured root, projecting it, and handing off the watcher are one atomic
/// operation from the perspective of every filesystem mutation.
pub(crate) async fn reconcile_disk_state_under_locks(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    _game_guard: &tokio::sync::MutexGuard<'_, ()>,
    _operation_guard: &crate::platform::fs::operation_lock::OpGuard,
) -> Result<DiskReconcileResult, AppError> {
    run_reconcile_with_owned_locks(context, request).await
}

/// Run one reconcile while a disk mutation retains its owned game and
/// operation locks. The lease is an ownership proof; this path must not try to
/// acquire either lock again.
pub(crate) async fn reconcile_disk_state_under_lease(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    _lease: &super::state::DiskMutationLease,
) -> Result<DiskReconcileResult, AppError> {
    run_reconcile_with_owned_locks(context, request).await
}

async fn run_reconcile_with_owned_locks(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
) -> Result<DiskReconcileResult, AppError> {
    let repair_evidence = request
        .watcher_session
        .as_ref()
        .and_then(|session| context.watcher_suppressor.pending_repair(session));
    let game_id = request.game_id;
    let may_reuse_activation_scan = request.force_full
        && request.changed_paths.is_empty()
        && request.watcher_events.is_empty()
        && request.path_hints.is_empty()
        && matches!(
            request.reason,
            crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::ModsViewEntered
                | crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::GameSwitched
        )
        && !context.watcher_suppressor.has_unrepaired_drops();
    if may_reuse_activation_scan {
        if let Some(result) = context
            .state
            .recent_applied_result(&game_id, std::time::Duration::from_secs(5))
        {
            return Ok(result);
        }
    }
    let force_full = request.force_full || repair_evidence.is_some();
    let result = run_refresh_once(RefreshRequest {
        context: context.clone(),
        game_id: &game_id,
        reason: request.reason,
        changed_paths: request.changed_paths,
        force_full,
        watcher_events: request.watcher_events,
        path_hints: request.path_hints,
    })
    .await?;

    if force_full && result.status.applied() {
        if let Some(evidence) = &repair_evidence {
            context.watcher_suppressor.mark_repaired_through(evidence);
        }
    }

    context.state.record_result(&game_id, &result);
    Ok(result)
}
