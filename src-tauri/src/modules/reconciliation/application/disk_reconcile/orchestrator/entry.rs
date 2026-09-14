//! Public entry points: serialize per game and run one reconcile request.

use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileResult, DiskReconcileScanScope,
};
use crate::modules::workspace::application::scanner::watcher::{
    ModWatchEvent, WatcherSession, WatcherState,
};
use crate::shared::errors::AppError;

use super::request::{DiskReconcileContext, DiskReconcileRequest};
use super::run::{run_refresh_once, RefreshRequest};

/// Outcome reserved for watcher-owned requests. A replaced watcher session is
/// safe to discard before it observes disk, mutates the projection, or emits
/// a reconcile result.
#[derive(Debug)]
pub(crate) enum WatcherReconcileOutcome {
    Applied(DiskReconcileResult),
    Superseded,
}

pub(super) fn watcher_outcome_for_current_session(
    watcher_state: &WatcherState,
    watcher_session: &WatcherSession,
    result: DiskReconcileResult,
) -> WatcherReconcileOutcome {
    if watcher_state.is_current_session(watcher_session) {
        WatcherReconcileOutcome::Applied(result)
    } else {
        WatcherReconcileOutcome::Superseded
    }
}

/// Reconcile one watcher request while proving its session remains current at
/// every lock boundary. The generic reconcile entry stays non-cancellable for
/// mutations and recovery callers that do not own a watcher session.
pub(crate) async fn reconcile_disk_state_for_watcher(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    watcher_state: &WatcherState,
    watcher_session: WatcherSession,
) -> Result<WatcherReconcileOutcome, AppError> {
    if !watcher_state.is_current_session(&watcher_session) {
        return Ok(WatcherReconcileOutcome::Superseded);
    }

    let request = request.for_watcher_session(watcher_session.clone());
    let game_lock = context.state.lock_for_game(&request.game_id);
    let game_guard = game_lock.lock().await;
    if !watcher_state.is_current_session(&watcher_session) {
        return Ok(WatcherReconcileOutcome::Superseded);
    }

    let operation_guard = context.operation_lock.acquire_for_reconcile().await;
    if !watcher_state.is_current_session(&watcher_session) {
        return Ok(WatcherReconcileOutcome::Superseded);
    }

    let result =
        reconcile_disk_state_under_locks(context, request, &game_guard, &operation_guard).await;
    let result = match result {
        Ok(result) => result,
        Err(error) if watcher_state.is_current_session(&watcher_session) => return Err(error),
        Err(_) => return Ok(WatcherReconcileOutcome::Superseded),
    };
    Ok(watcher_outcome_for_current_session(
        watcher_state,
        &watcher_session,
        result,
    ))
}

/// Disk Reconcile watcher batches must stay disk-only.
/// Watcher must never invoke the Deep Match Scanner pipeline.
pub(crate) async fn reconcile_disk_state_from_watcher_batch(
    context: DiskReconcileContext<'_>,
    game_id: String,
    mods_path: &std::path::Path,
    changed_paths: Vec<String>,
    watcher_events: &[ModWatchEvent],
    watcher_state: &WatcherState,
    watcher_session: WatcherSession,
) -> Result<WatcherReconcileOutcome, AppError> {
    reconcile_disk_state_for_watcher(
        context,
        DiskReconcileRequest::watcher_batch(game_id, mods_path, changed_paths, watcher_events),
        watcher_state,
        watcher_session,
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
            crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ModsViewEntered
                | crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::GameSwitched
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
    let mut result = run_refresh_once(RefreshRequest {
        context: context.clone(),
        game_id: &game_id,
        reason: request.reason,
        changed_paths: request.changed_paths,
        force_full,
        watcher_events: request.watcher_events,
        path_hints: request.path_hints,
        defer_overlay_sync: request.defer_overlay_sync,
        precomputed_discovery: request.precomputed_discovery,
    })
    .await?;

    if result.status.applied() && result.scan_scope == DiskReconcileScanScope::Full {
        if let Some(evidence) = &repair_evidence {
            context.watcher_suppressor.mark_repaired_through(evidence);
        }
    }

    context.state.record_result(&game_id, &mut result);
    Ok(result)
}
