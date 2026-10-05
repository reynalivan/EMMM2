//! Public entry points: serialize per game and run one reconcile request.

use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileResult, DiskReconcileScanScope,
};
use crate::modules::workspace::application::scanner::watcher::{WatcherSession, WatcherState};
use crate::shared::errors::AppError;

use super::request::{DiskReconcileContext, DiskReconcileRequest};
use super::run::{run_refresh_once, RefreshRequest};

/// Outcome reserved for watcher-owned requests. A replaced watcher session is
/// safe to discard before it observes disk, mutates the projection, or emits
/// a reconcile result.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
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

/// Internal reconciliation keeps the watcher proof and the projected revision
/// in one serialization window. A dirty watcher token requires a full pass.
pub(crate) async fn reconcile_disk_state_with_authority(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    watcher_state: &WatcherState,
    mods_root: &std::path::Path,
) -> Result<DiskReconcileResult, AppError> {
    let game_lock = context.state.lock_for_game(&request.game_id);
    let _game_guard = game_lock.lock().await;
    let _operation_guard = context.operation_lock.acquire_for_reconcile().await;
    if let Some(generation) = request.initial_recovery_generation {
        if !context
            .state
            .initial_recovery_generation_is_pending(&request.game_id, generation)
        {
            return Err(AppError::Cancelled);
        }
    }
    run_reconcile_with_authority(context, request, watcher_state, mods_root).await
}

pub(crate) async fn reconcile_disk_state_under_owned_game_lock_with_authority(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    watcher_state: &WatcherState,
    mods_root: &std::path::Path,
    _game_guard: &tokio::sync::OwnedMutexGuard<()>,
    _operation_guard: &crate::platform::fs::operation_lock::OpGuard,
) -> Result<DiskReconcileResult, AppError> {
    run_reconcile_with_authority(context, request, watcher_state, mods_root).await
}

pub(crate) async fn reconcile_disk_state_under_lease_with_authority(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    watcher_state: &WatcherState,
    mods_root: &std::path::Path,
    _lease: &super::state::DiskMutationLease,
) -> Result<DiskReconcileResult, AppError> {
    run_reconcile_with_authority(context, request, watcher_state, mods_root).await
}

async fn run_reconcile_with_authority(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
    watcher_state: &WatcherState,
    mods_root: &std::path::Path,
) -> Result<DiskReconcileResult, AppError> {
    let game_id = request.game_id.clone();
    let reason = request.reason.clone();
    let defer_overlay_sync = request.defer_overlay_sync;
    let allow_unproven_initial_pass = matches!(
        context.state.initial_recovery_readiness(&game_id),
        super::state::InitialRecoveryReadiness::Unstarted { .. }
            | super::state::InitialRecoveryReadiness::Syncing { .. }
    );
    let mut first_request = Some(request);
    for attempt in 0..2 {
        let mut current_request = first_request.take().unwrap_or_else(|| {
            let full =
                DiskReconcileRequest::manual(game_id.clone(), reason.clone(), Vec::new(), true);
            if defer_overlay_sync {
                full.defer_overlay_sync()
            } else {
                full
            }
        });
        let marker = watcher_state
            .current_session_for_root(mods_root)
            .and_then(|session| {
                context
                    .state
                    .authority_event_generation(&game_id, session.generation())
                    .map(|generation| (session, generation))
            });
        let repair_evidence = marker
            .as_ref()
            .and_then(|(session, _)| context.watcher_suppressor.pending_repair(session));
        if let Some((session, _)) = &marker {
            if repair_evidence.is_some()
                || !matches!(
                    context
                        .state
                        .authority_catch_up(&game_id, mods_root, session.generation()),
                    super::state::AuthorityCatchUp::Clean { .. }
                )
            {
                current_request.force_full = true;
                current_request.trusted_mutation_scope = false;
            }
        }
        let changed_paths = current_request.changed_paths.clone();
        let echo_watermark = context.watcher_suppressor.expected_echo_watermark();
        let mut result = run_reconcile_with_owned_locks(context.clone(), current_request).await?;
        if !result.status.applied() {
            return Ok(result);
        }
        match marker {
            Some((session, generation)) if watcher_state.is_current_session(&session) => {
                if context.state.mark_authority_reconciled(
                    &game_id,
                    mods_root,
                    session.generation(),
                    generation,
                    &result,
                    &changed_paths,
                ) {
                    if result.scan_scope == DiskReconcileScanScope::Full {
                        if let Some(evidence) = &repair_evidence {
                            context.watcher_suppressor.mark_repaired_through(evidence);
                        }
                    }
                    context
                        .watcher_suppressor
                        .mark_rename_echoes_reconciled_through(&session, echo_watermark);
                    return Ok(result);
                }
            }
            None if allow_unproven_initial_pass => return Ok(result),
            _ => {}
        }
        if attempt == 1 {
            result.warnings.push(
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileWarning {
                    kind: crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileWarningKind::AuthorityPending,
                    message: "Disk projection succeeded, but watcher validation is still pending"
                        .to_string(),
                },
            );
            return Ok(result);
        }
    }
    Err(AppError::Internal(
        "Disk authority retry ended without a result".to_string(),
    ))
}

/// Best-effort background reconciliation. It never waits for either
/// serialization lock, so foreground work can retry it after its storage commit.
pub(crate) async fn try_reconcile_disk_state_for_prewarm(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
) -> Result<Option<DiskReconcileResult>, AppError> {
    let game_lock = context.state.lock_for_game(&request.game_id);
    let Ok(_game_guard) = game_lock.try_lock_owned() else {
        return Ok(None);
    };
    let Some(_operation_guard) = context.operation_lock.try_acquire_for_reconcile() else {
        return Ok(None);
    };
    let operation_lock = context.operation_lock;
    tokio::select! {
        biased;
        () = operation_lock.wait_for_foreground_intent() => Ok(None),
        result = run_reconcile_with_owned_locks(context, request) => result.map(Some),
    }
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

async fn run_reconcile_with_owned_locks(
    context: DiskReconcileContext<'_>,
    request: DiskReconcileRequest,
) -> Result<DiskReconcileResult, AppError> {
    let repair_evidence = request
        .watcher_session
        .as_ref()
        .and_then(|session| context.watcher_suppressor.pending_repair(session));
    let game_id = request.game_id;
    let force_full = request.force_full || repair_evidence.is_some();
    let mut result = run_refresh_once(RefreshRequest {
        context: context.clone(),
        game_id: &game_id,
        reason: request.reason,
        changed_paths: request.changed_paths,
        force_full,
        watcher_events: request.watcher_events,
        path_hints: request.path_hints,
        trusted_mutation_scope: request.trusted_mutation_scope,
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
