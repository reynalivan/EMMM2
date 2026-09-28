//! Watcher lifecycle management.
//!
//! The watcher is now a pure trigger source:
//! - collect filesystem events
//! - debounce into batches
//! - delegate Disk Reconcile to `disk_reconcile`
//! - emit typed payloads back to the frontend

use crate::modules::workspace::application::scanner::watcher::{
    ModWatchEvent, WatchEventPayload, WatcherSession, WatcherState, WatcherSuppressor,
};
use crate::shared::errors::ScannerError;
use crate::shared::sync::lock;
use std::sync::Arc;
use tauri::{Emitter, Manager};

const INACTIVE_PREWARM_BUDGET: std::time::Duration = std::time::Duration::from_millis(250);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatcherInstallScope {
    InactiveOnly,
    StartupSelected,
}

fn install_scope_matches_selection(
    scope: WatcherInstallScope,
    selected_game_id: Option<&str>,
    game_id: &str,
) -> bool {
    match scope {
        WatcherInstallScope::InactiveOnly => selected_game_id != Some(game_id),
        WatcherInstallScope::StartupSelected => selected_game_id == Some(game_id),
    }
}

fn initial_reconcile_reason(
    activation: bool,
) -> crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason {
    if activation {
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::GameSwitched
    } else {
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WatcherActivation {
    pub activation_generation: u64,
    pub recovery_generation: u64,
}

struct WatcherLoopContext {
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
    mods_path_root: String,
    suppressor: Arc<WatcherSuppressor>,
    session: WatcherSession,
    activation: Option<WatcherActivation>,
}

fn emit_event(app: &tauri::AppHandle, payload: WatchEventPayload) {
    let _ = app.emit("mod_watch:event", payload);
}

fn no_pending_disk_commit(
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
) -> bool {
    match coordinator.has_pending_disk_commit_for_game(game_id) {
        Ok(pending) => !pending,
        Err(error) => {
            log::warn!("Could not check pending disk commit for '{game_id}': {error}");
            false
        }
    }
}

fn clean_cached_activation_result(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
) -> Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>
{
    let _game_guard = state.try_game_guard(game_id)?;
    if !no_pending_disk_commit(coordinator, game_id) || suppressor.pending_repair(session).is_some()
    {
        return None;
    }
    let result = state.clean_authoritative_result(game_id, mods_root, session.generation())?;
    suppressor
        .pending_repair(session)
        .is_none()
        .then_some(result)
}

pub(crate) fn inactive_activation_is_ready(
    app: &tauri::AppHandle,
    watcher: &WatcherState,
    game_id: &str,
    mods_root: &std::path::Path,
    runtime_config_path: &std::path::Path,
) -> bool {
    let Some(generation) =
        watcher.inactive_watcher_session(game_id, mods_root, Some(runtime_config_path))
    else {
        return false;
    };
    let session =
        WatcherSession::new_with_runtime_config(generation, mods_root, Some(runtime_config_path));
    let state = app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
    let coordinator = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
    matches!(
        state.initial_recovery_readiness(game_id),
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Ready { .. }
    ) && clean_cached_activation_result(
        state.inner(), watcher.suppressor.as_ref(), coordinator.inner(),
        game_id, mods_root, &session,
    ).is_some()
}

fn prepare_activation_recovery(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
    activation_generation: u64,
) -> (
    WatcherActivation,
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
) {
    let cached =
        clean_cached_activation_result(state, suppressor, coordinator, game_id, mods_root, session);
    if let (Some(result), crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Ready { generation }) =
        (cached, state.initial_recovery_readiness(game_id))
    {
        return (WatcherActivation { activation_generation, recovery_generation: generation }, Some(result));
    }
    if !matches!(
        state.initial_recovery_readiness(game_id),
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Unstarted { .. }
    ) {
        state.reset_initial_recovery(game_id);
    }
    let recovery_generation = state.mark_initial_recovery_pending(game_id);
    (
        WatcherActivation {
            activation_generation,
            recovery_generation,
        },
        None,
    )
}

fn prepare_inactive_authority(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &str,
    mods_root: &std::path::Path,
    covered_session: Option<u64>,
    watcher_session: u64,
) -> bool {
    let transferred = covered_session.is_some_and(|covered_session| {
        state.handoff_authority_session(game_id, mods_root, covered_session, watcher_session)
    });
    if !transferred
        && state
            .authority_event_generation(game_id, watcher_session)
            .is_none()
    {
        state.begin_authority_session(game_id, mods_root, watcher_session);
    }
    transferred
}

fn initial_watcher_recovery_plan(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    suppressor: &WatcherSuppressor,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
) -> (
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
    Vec<String>,
    bool,
    u64,
) {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp;

    let watcher_session = session.generation();
    match state.authority_catch_up(game_id, mods_root, watcher_session) {
        AuthorityCatchUp::Clean {
            observed_generation,
            ..
        } if state.authority_event_generation(game_id, watcher_session)
            == Some(observed_generation) =>
        {
            let cached = clean_cached_activation_result(
                state,
                suppressor,
                coordinator,
                game_id,
                mods_root,
                session,
            );
            let force_full = cached.is_none();
            (cached, Vec::new(), force_full, observed_generation)
        }
        AuthorityCatchUp::Scoped {
            changed_paths,
            observed_generation,
        } => (None, changed_paths, false, observed_generation),
        AuthorityCatchUp::Full {
            observed_generation,
        }
        | AuthorityCatchUp::Clean {
            observed_generation,
            ..
        } => (None, Vec::new(), true, observed_generation),
    }
}

async fn prewarm_without_activation_guard<T>(
    guard: tokio::sync::MutexGuard<'_, ()>,
    work: impl std::future::Future<Output = T>,
) -> T {
    drop(guard);
    work.await
}

struct PrewarmAcceptance<'a> {
    game_id: &'a str,
    mods_root: &'a std::path::Path,
    runtime_config_path: Option<&'a std::path::Path>,
    watcher_session: u64,
    observed_generation: u64,
    result:
        &'a crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    changed_paths: &'a [String],
}

async fn accept_prewarm_result(
    state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    watcher: &WatcherState,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
    selected_game_id: impl FnOnce() -> Option<String>,
    acceptance: PrewarmAcceptance<'_>,
) -> bool {
    let PrewarmAcceptance {
        game_id,
        mods_root,
        runtime_config_path,
        watcher_session,
        observed_generation,
        result,
        changed_paths,
    } = acceptance;
    let _activation_guard = state.activation_guard().await;
    let Some(_game_guard) = state.try_game_guard(game_id) else {
        state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
        return false;
    };
    let session =
        WatcherSession::new_with_runtime_config(watcher_session, mods_root, runtime_config_path);
    let active_coverage = selected_game_id().as_deref() == Some(game_id)
        && watcher
            .current_session_for_coverage(mods_root, runtime_config_path)
            .is_some_and(|session| session.generation() == watcher_session);
    let inactive_coverage =
        watcher.inactive_watcher_session(game_id, mods_root, runtime_config_path)
            == Some(watcher_session);
    let trusted = (active_coverage || inactive_coverage)
        && no_pending_disk_commit(coordinator, game_id)
        && watcher.suppressor.pending_repair(&session).is_none()
        && state.mark_authority_reconciled(
            game_id,
            mods_root,
            watcher_session,
            observed_generation,
            result,
            changed_paths,
        );
    if !trusted {
        state.reject_untrusted_reconcile(game_id, result.reconcile_revision);
    }
    trusted
}

fn settle_pending_onboarding_indexing_after_activation(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) {
    use crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase;

    let phase = if result.status.applied() {
        OnboardingIndexingBackgroundPhase::Ready
    } else {
        OnboardingIndexingBackgroundPhase::NeedsAttention
    };
    let sessions = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
    >();
    let Some(status) = sessions.set_background_phase_for_game(game_id, phase) else {
        return;
    };
    if let Err(error) = app.emit("onboarding_indexing:background_status", status) {
        log::debug!("Could not emit activation onboarding indexing status: {error}");
    }
    if !result.status.applied() {
        return;
    }
    let pool = pool.clone();
    let game_id = game_id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = crate::modules::reconciliation::application::disk_reconcile::onboarding_recovery::remove_pending_game_id(
            &pool,
            &game_id,
        )
        .await
        {
            log::warn!("Could not clear completed onboarding game after activation: {error}");
        }
    });
}

fn enqueue_runtime_sync_after_watcher_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    events_lost: bool,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) {
    if !result.status.applied() || (!result.folders_changed && !result.runtime_file_changed) {
        return;
    }
    if app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .get_settings()
        .active_game_id
        .as_deref()
        != Some(game_id)
    {
        return;
    }

    if events_lost || result.changed_roots.is_empty() {
        crate::modules::reconciliation::api::enqueue_runtime_sync(
            app,
            pool,
            game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
        );
        return;
    }

    crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
        app,
        pool,
        game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
        crate::modules::reconciliation::api::runtime_sync_request_for_roots(&result.changed_roots),
    );
}

fn authority_observer(
    app: &tauri::AppHandle,
    game_id: &str,
    mods_root: &std::path::Path,
    session: &WatcherSession,
    suppressor: Arc<WatcherSuppressor>,
) -> crate::modules::workspace::application::scanner::watcher::WatchObservation {
    let app = app.clone();
    let game_id = game_id.to_string();
    let mods_root = mods_root.to_path_buf();
    let watcher_session = session.generation();
    let session = session.clone();
    std::sync::Arc::new(move |paths, kind, events_lost| {
        let state = app.state::<
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
        >();
        if !events_lost
            && kind.is_some_and(|kind| {
                suppressor.consume_expected_rename_echo(&game_id, &session, kind, paths)
            })
        {
            return;
        }
        state.observe_authority_event(&game_id, watcher_session, &mods_root, paths, events_lost);
    })
}

pub(crate) fn start_inactive_watcher(
    app: &tauri::AppHandle,
    state: &WatcherState,
    game_id: &str,
    mods_root: &std::path::Path,
    runtime_config_path: Option<&std::path::Path>,
) -> Result<Option<u64>, ScannerError> {
    let reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let Some(activation_guard) = reconcile_state.try_activation_guard() else {
        return Ok(None);
    };
    start_inactive_watcher_with_activation_guard(
        app,
        state,
        game_id,
        mods_root,
        runtime_config_path,
        &activation_guard,
        WatcherInstallScope::InactiveOnly,
    )
}

pub(crate) fn start_inactive_watcher_with_activation_guard(
    app: &tauri::AppHandle,
    state: &WatcherState,
    game_id: &str,
    mods_root: &std::path::Path,
    runtime_config_path: Option<&std::path::Path>,
    _activation_guard: &tokio::sync::MutexGuard<'_, ()>,
    scope: WatcherInstallScope,
) -> Result<Option<u64>, ScannerError> {
    let reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let active_watcher = lock(&state.watcher);
    if !install_scope_matches_selection(
        scope,
        app.state::<crate::modules::settings::application::config::ConfigService>()
            .get_settings()
            .active_game_id
            .as_deref(),
        game_id,
    ) {
        return Ok(None);
    }
    if let Some(session) = state.inactive_watcher_session(game_id, mods_root, runtime_config_path) {
        return Ok(Some(session));
    }
    if state.discard_inactive_watcher_unless_coverage(game_id, mods_root, runtime_config_path) {
        reconcile_state.invalidate_authority(game_id, mods_root);
    }
    let covered_session = active_watcher
        .as_ref()
        .and_then(|_| state.current_session_for_coverage(mods_root, runtime_config_path))
        .map(|session| session.generation());
    let session = state.prepare_session_with_runtime_config(mods_root, runtime_config_path);
    let observer = authority_observer(app, game_id, mods_root, &session, state.suppressor.clone());
    let watcher_result = crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config_and_observer(
        mods_root,
        runtime_config_path,
        state.suppressor.clone(),
        session.clone(),
        Some(observer),
    );
    let (watcher, receiver) = match watcher_result {
        Ok(installed) => installed,
        Err(error) => {
            app.state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >()
            .invalidate_authority(game_id, mods_root);
            return Err(error);
        }
    };
    state.install_inactive_watcher(
        game_id.to_string(),
        mods_root,
        runtime_config_path,
        session.clone(),
        watcher,
        receiver,
    );
    prepare_inactive_authority(
        reconcile_state.inner(),
        game_id,
        mods_root,
        covered_session,
        session.generation(),
    );
    drop(active_watcher);
    Ok(Some(session.generation()))
}

fn prewarm_inactive_games(
    app: &tauri::AppHandle,
    active_game_id: &str,
    activation_generation: u64,
) {
    let app = app.clone();
    let active_game_id = active_game_id.to_string();
    tokio::spawn(async move {
        let games = app
            .state::<crate::modules::settings::application::config::ConfigService>()
            .get_settings()
            .games;
        for game in games {
            if game.id == active_game_id {
                continue;
            }
            if app.state::<crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>()
                .has_unfinished_game(&game.id)
            {
                continue;
            }
            let reconcile_state = app.state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >();
            if !reconcile_state.activation_is_current(Some(&active_game_id), activation_generation)
            {
                return;
            }
            if !game.mod_path.is_dir() {
                continue;
            }
            let watcher_state = app.state::<WatcherState>();
            let runtime_config_path = game.instance_path.join("d3dx.ini");
            let watcher_session = match start_inactive_watcher(
                &app,
                watcher_state.inner(),
                &game.id,
                &game.mod_path,
                Some(&runtime_config_path),
            ) {
                Ok(Some(session)) => session,
                Ok(None) => continue,
                Err(error) => {
                    log::warn!("Could not prewarm watcher for '{}': {error}", game.id);
                    continue;
                }
            };
            let Some(activation_guard) = reconcile_state.try_activation_guard() else {
                continue;
            };
            if !reconcile_state.activation_is_current(Some(&active_game_id), activation_generation)
            {
                return;
            }
            let catch_up =
                reconcile_state.authority_catch_up(&game.id, &game.mod_path, watcher_session);
            let (changed_paths, force_full, observed_generation) = match catch_up {
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Clean { .. } => {
                    continue;
                }
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Scoped {
                    changed_paths,
                    observed_generation,
                } => (changed_paths, false, observed_generation),
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Full {
                    observed_generation,
                } => (Vec::new(), true, observed_generation),
            };
            let pool = app.state::<sqlx::SqlitePool>();
            let config =
                app.state::<crate::modules::settings::application::config::ConfigService>();
            let operation_lock =
                app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
            let result = prewarm_without_activation_guard(activation_guard, tokio::time::timeout(
                INACTIVE_PREWARM_BUDGET,
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::try_reconcile_disk_state_for_prewarm(
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                        pool: pool.inner(),
                        config: config.inner(),
                        state: reconcile_state.inner(),
                        watcher_suppressor: watcher_state.suppressor.clone(),
                        operation_lock: operation_lock.inner_lock(),
                        progress_reporter: None,
                    },
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                        game.id.clone(),
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::StartupBoot,
                        changed_paths.clone(),
                        force_full,
                    )
                    .defer_overlay_sync(),
                ),
            )).await;
            match result {
                Ok(Ok(Some(result))) => {
                    accept_prewarm_result(
                        reconcile_state.inner(),
                        watcher_state.inner(),
                        operation_lock.inner(),
                        || config.get_settings().active_game_id,
                        PrewarmAcceptance {
                            game_id: &game.id,
                            mods_root: &game.mod_path,
                            runtime_config_path: Some(&runtime_config_path),
                            watcher_session,
                            observed_generation,
                            result: &result,
                            changed_paths: &changed_paths,
                        },
                    )
                    .await;
                }
                Ok(Ok(None)) => {
                    log::debug!(
                        "Skipping inactive projection prewarm for '{}' because a foreground operation is busy",
                        game.id
                    );
                }
                Ok(Err(error)) => {
                    log::warn!("Could not prewarm projection for '{}': {error}", game.id);
                }
                Err(_) => log::debug!(
                    "Deferring inactive projection prewarm for '{}' after {}ms budget",
                    game.id,
                    INACTIVE_PREWARM_BUDGET.as_millis()
                ),
            }
            tokio::task::yield_now().await;
        }
    });
}

fn emit_reconcile_result_for_current_session(
    app: &tauri::AppHandle,
    session: &WatcherSession,
    result: crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) -> bool {
    app.state::<WatcherState>()
        .with_current_session(session, || app.emit("disk_reconcile:result", result))
        .is_some()
}

fn enqueue_telemetry(
    app: &tauri::AppHandle,
    events: impl IntoIterator<Item = crate::modules::system::application::telemetry::TelemetryEvent>,
) {
    if !app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .get_settings()
        .diagnostics
        .telemetry_enabled
    {
        return;
    }
    if let Some(sink) =
        app.try_state::<crate::modules::system::application::telemetry::TelemetrySink>()
    {
        sink.inner().try_enqueue(events);
    }
}

fn publish_activation_result(
    app: &tauri::AppHandle,
    game_id: &str,
    activation: WatcherActivation,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) {
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        DiskReconcileStatus, GameActivationPhase, GameActivationStatus,
    };

    let state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let Some(authority) =
        state.activation_authority_for_generation(game_id, activation.activation_generation)
    else {
        return;
    };
    let phase = match result.status {
        DiskReconcileStatus::Applied | DiskReconcileStatus::AppliedWithFolderConflicts => {
            GameActivationPhase::Ready
        }
        DiskReconcileStatus::SourceUnavailable => GameActivationPhase::SourceUnavailable,
        DiskReconcileStatus::NeedsRenameConfirmation => GameActivationPhase::Failed,
    };
    let is_ready = matches!(phase, GameActivationPhase::Ready);
    let runtime_authority = authority.clone();
    let _ = authority.with_current(|| {
        let runtime_sync_generation = is_ready.then(|| {
            crate::modules::reconciliation::api::enqueue_runtime_sync_with_authority(
                app,
                app.state::<sqlx::SqlitePool>().inner(),
                game_id,
                crate::modules::reconciliation::api::RuntimeSyncCause::GameActivated,
                runtime_authority,
            )
        });
        let status = GameActivationStatus {
            game_id: Some(game_id.to_string()),
            generation: activation.activation_generation,
            phase,
            reconcile_revision: Some(result.reconcile_revision),
            runtime_sync_generation,
            error: result.error_message.clone(),
        };
        if let Err(error) = app.emit("game_activation:status", status) {
            log::warn!("Could not emit game activation status: {error}");
        }
        if is_ready {
            prewarm_inactive_games(app, game_id, activation.activation_generation);
        }
    });
}

fn emit_activation_failure(
    app: &tauri::AppHandle,
    game_id: &str,
    activation: WatcherActivation,
    error: String,
) {
    let state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let Some(authority) =
        state.activation_authority_for_generation(game_id, activation.activation_generation)
    else {
        return;
    };
    let _ = authority.with_current(|| {
        let status = crate::modules::reconciliation::application::disk_reconcile::types::GameActivationStatus {
            game_id: Some(game_id.to_string()),
            generation: activation.activation_generation,
            phase: crate::modules::reconciliation::application::disk_reconcile::types::GameActivationPhase::Failed,
            reconcile_revision: None,
            runtime_sync_generation: None,
            error: Some(error),
        };
        if let Err(error) = app.emit("game_activation:status", status) {
            log::warn!("Could not emit game activation failure: {error}");
        }
    });
}

fn replace_watcher(
    state: &WatcherState,
    root: &std::path::Path,
    runtime_config_path: Option<&std::path::Path>,
    build: impl FnOnce(
        WatcherSession,
    ) -> Result<
        (
            crate::modules::workspace::application::scanner::watcher::ModWatcher,
            crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        ),
        ScannerError,
    >,
) -> Result<
    (
        WatcherSession,
        crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        Option<u64>,
    ),
    ScannerError,
> {
    let mut active_watcher = lock(&state.watcher);
    let covered_session = active_watcher
        .as_ref()
        .and_then(|_| state.current_session_for_coverage(root, runtime_config_path))
        .map(|session| session.generation());
    let session = state.prepare_session_with_runtime_config(root, runtime_config_path);
    let (watcher, receiver) = build(session.clone())?;
    state.publish_session(&session);
    if active_watcher.is_some() {
        log::info!("Stopping existing watcher");
    }
    *active_watcher = Some(watcher);
    Ok((session, receiver, covered_session))
}

pub fn start_watcher(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
) -> Result<(), ScannerError> {
    start_watcher_inner(app, state, pool, path, game_id, None).map(|_| ())
}

pub fn start_watcher_for_activation(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
    activation_generation: u64,
) -> Result<
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
    ScannerError,
> {
    start_watcher_inner(app, state, pool, path, game_id, Some(activation_generation))
}

fn start_watcher_inner(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
    activation_generation: Option<u64>,
) -> Result<
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
    ScannerError,
> {
    let path_obj = std::path::Path::new(&path);
    let runtime_config_path = app
        .state::<crate::modules::settings::application::config::ConfigService>()
        .get_settings()
        .games
        .into_iter()
        .find(|game| game.id == game_id)
        .map(|game| game.instance_path.join("d3dx.ini"));
    if state.discard_inactive_watcher_unless_coverage(
        &game_id,
        path_obj,
        runtime_config_path.as_deref(),
    ) {
        app.state::<
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
        >()
        .invalidate_authority(&game_id, path_obj);
    }

    log::info!("Starting watcher on: {}", path);

    let disk_reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let adopted = {
        let mut active_watcher = lock(&state.watcher);
        state
            .take_inactive_watcher_for_handoff(&game_id, path_obj, runtime_config_path.as_deref())
            .map(|(session, watcher, receiver)| {
                state.publish_session(&session);
                *active_watcher = Some(watcher);
                (session, receiver)
            })
    };
    let (session, rx) = if let Some(adopted) = adopted {
        adopted
    } else {
        let replacement = replace_watcher(
            state,
            path_obj,
            runtime_config_path.as_deref(),
            |session| {
                let observer = authority_observer(
                    &app,
                    &game_id,
                    path_obj,
                    &session,
                    state.suppressor.clone(),
                );
                crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config_and_observer(
                path_obj,
                runtime_config_path.as_deref(),
                state.suppressor.clone(),
                session,
                Some(observer),
            )
            },
        );
        let (session, receiver, _) = match replacement {
            Ok(replacement) => replacement,
            Err(error) => {
                let inactive_coverage = state
                    .inactive_watcher_session(&game_id, path_obj, runtime_config_path.as_deref())
                    .is_some_and(|watcher_session| {
                        disk_reconcile_state
                            .authority_event_generation(&game_id, watcher_session)
                            .is_some()
                    });
                let active_coverage = state
                    .current_session_for_coverage(path_obj, runtime_config_path.as_deref())
                    .is_some_and(|session| {
                        disk_reconcile_state
                            .authority_event_generation(&game_id, session.generation())
                            .is_some()
                    });
                if !inactive_coverage && !active_coverage {
                    disk_reconcile_state.invalidate_authority(&game_id, path_obj);
                }
                return Err(error);
            }
        };
        let _ = state.take_inactive_watcher_for_handoff(
            &game_id,
            path_obj,
            runtime_config_path.as_deref(),
        );
        disk_reconcile_state.begin_authority_session(&game_id, path_obj, session.generation());
        (session, receiver)
    };

    let activation_result = activation_generation.map(|generation| {
        let coordinator = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
        prepare_activation_recovery(
            disk_reconcile_state.inner(),
            state.suppressor.as_ref(),
            coordinator.inner(),
            &game_id,
            path_obj,
            &session,
            generation,
        )
    });
    if let Some((activation, cached)) = &activation_result {
        use crate::modules::reconciliation::application::disk_reconcile::types::{
            GameActivationPhase, GameActivationStatus,
        };
        let status = GameActivationStatus {
            game_id: Some(game_id.clone()),
            generation: activation.activation_generation,
            phase: if cached.is_some() {
                GameActivationPhase::Ready
            } else {
                GameActivationPhase::Syncing
            },
            reconcile_revision: cached.as_ref().map(|result| result.reconcile_revision),
            runtime_sync_generation: None,
            error: None,
        };
        if let Err(error) = app.emit("game_activation:status", status) {
            log::warn!("Could not emit game activation status: {error}");
        }
    }
    let reused = activation_result
        .as_ref()
        .and_then(|(_, cached)| cached.clone());
    let activation = activation_result.map(|(activation, _)| activation);
    let app_handle = app.clone();
    let db_pool = pool;
    let mods_path_root = path;
    let suppressor = state.suppressor.clone();

    tokio::spawn(async move {
        process_event_loop(
            rx,
            WatcherLoopContext {
                app: app_handle,
                pool: db_pool,
                game_id,
                mods_path_root,
                suppressor,
                session,
                activation,
            },
        )
        .await;
    });

    Ok(reused)
}

async fn process_event_loop(
    mut rx: crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
    context: WatcherLoopContext,
) {
    let WatcherLoopContext {
        app,
        pool,
        game_id,
        mods_path_root,
        suppressor,
        session,
        activation,
    } = context;
    if !app.state::<WatcherState>().is_current_session(&session) {
        return;
    }
    // Startup and activation can both inherit continuous watcher coverage.
    let disk_reconcile_state =
        app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
    let config = app.state::<crate::modules::settings::application::config::ConfigService>();
    let operation_lock = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
    let watcher_state = app.state::<WatcherState>();
    let mods_root = std::path::Path::new(&mods_path_root);
    let watcher_session = session.generation();
    let initial_reconcile_reason = initial_reconcile_reason(activation.is_some());
    let (cached_result, recovery_changed_paths, recovery_force_full, recovery_generation) =
        initial_watcher_recovery_plan(
            disk_reconcile_state.inner(),
            suppressor.as_ref(),
            operation_lock.inner(),
            &game_id,
            mods_root,
            &session,
        );
    let (onboarding_lease, activation_claim_guard) = if activation.is_some()
        && cached_result.is_none()
    {
        let sessions = app.state::<
            crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
        >();
        sessions.promote_game(&game_id);
        if let Some((claimed, guard)) = sessions.claim_for_activation(&game_id) {
            match claimed.resolve().await {
                Ok(crate::modules::reconciliation::application::disk_reconcile::onboarding_session::ConsumedOnboardingSnapshot::Snapshot(lease))
                    if lease.matches_root(mods_root)
                        && lease.journal_allows_game_snapshot(operation_lock.inner(), &game_id) =>
                {
                    (Some(lease), Some(guard))
                }
                Ok(_) => (None, Some(guard)),
                Err(error) => {
                    log::warn!("Onboarding snapshot for '{game_id}' needs a full recheck: {error}");
                    (None, Some(guard))
                }
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    let joined_background_result = if activation.is_some()
        && cached_result.is_none()
        && onboarding_lease.is_none()
        && activation_claim_guard.is_none()
        && recovery_force_full
    {
        let sessions = app.state::<
            crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
        >();
        let previous_revision = disk_reconcile_state
            .authoritative_result(&game_id)
            .map(|result| result.reconcile_revision)
            .unwrap_or(0);
        let background_ready = matches!(
            sessions.wait_for_background_claimed_game(&game_id).await,
            Some(crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::Ready)
        );
        let acceptance_guard = operation_lock.inner_lock().try_acquire_for_reconcile();
        if background_ready
            && acceptance_guard.is_some()
            && no_pending_disk_commit(operation_lock.inner(), &game_id)
            && watcher_state.is_current_session(&session)
            && suppressor.pending_repair(&session).is_none()
            && disk_reconcile_state.authority_event_generation(&game_id, watcher_session)
                == Some(recovery_generation)
        {
            disk_reconcile_state.authoritative_result(&game_id).and_then(|mut result| {
                if result.status.applied() && result.reconcile_revision > previous_revision {
                    let trusted = disk_reconcile_state.mark_authority_reconciled(
                        &game_id,
                        mods_root,
                        watcher_session,
                        recovery_generation,
                        &result,
                        &[],
                    );
                    if trusted {
                        result.scan_scope = crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::None;
                        Some(result)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        } else {
            None
        }
    } else {
        None
    };
    let session_recovery = if let Some(result) = cached_result.or(joined_background_result) {
        Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result))
    } else {
        let recovery_paths = recovery_changed_paths.clone();
        let mut request = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
            game_id.clone(),
            initial_reconcile_reason.clone(),
            recovery_paths,
            recovery_force_full,
        )
        .defer_overlay_sync();
        if let Some(lease) = &onboarding_lease {
            request = request.with_precomputed_discovery(lease.discovery());
        }
        let mut outcome = crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_for_watcher(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &pool,
                config: config.inner(),
                state: disk_reconcile_state.inner(),
                watcher_suppressor: suppressor.clone(),
                operation_lock: operation_lock.inner_lock(),
                progress_reporter: Some(std::sync::Arc::new(
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                        app.clone(),
                        game_id.clone(),
                        initial_reconcile_reason.clone(),
                    )
                    .for_watcher_session(session.clone()),
                )),
            },
            request,
            watcher_state.inner(),
            session.clone(),
        )
        .await;
        if matches!(&outcome, Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(_)))
            && onboarding_lease
                .as_ref()
                .is_some_and(|lease| !lease.journal_allows_game_snapshot(operation_lock.inner(), &game_id))
        {
            outcome = crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_for_watcher(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                    pool: &pool,
                    config: config.inner(),
                    state: disk_reconcile_state.inner(),
                    watcher_suppressor: suppressor.clone(),
                    operation_lock: operation_lock.inner_lock(),
                    progress_reporter: None,
                },
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                    game_id.clone(),
                    initial_reconcile_reason,
                    Vec::new(),
                    true,
                )
                .defer_overlay_sync(),
                watcher_state.inner(),
                session.clone(),
            )
            .await;
        }
        if let Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result)) = &outcome {
            disk_reconcile_state.mark_authority_reconciled(
                &game_id,
                mods_root,
                watcher_session,
                recovery_generation,
                result,
                &recovery_changed_paths,
            );
        }
        outcome
    };
    let session_recovery = if activation.is_some() {
        match session_recovery {
            Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(initial_result)) => {
                let mut buffered_events = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    buffered_events.push(event);
                }
                if buffered_events.is_empty() {
                    Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(initial_result))
                } else {
                    let changed_paths = crate::modules::reconciliation::application::disk_reconcile::watcher_batch::collect_changed_paths(&buffered_events);
                    let observed_generation = disk_reconcile_state
                        .authority_event_generation(&game_id, watcher_session)
                        .unwrap_or(0);
                    let outcome = crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_for_watcher(
                        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                            pool: &pool,
                            config: config.inner(),
                            state: disk_reconcile_state.inner(),
                            watcher_suppressor: suppressor.clone(),
                            operation_lock: operation_lock.inner_lock(),
                            progress_reporter: Some(std::sync::Arc::new(
                                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                                    app.clone(),
                                    game_id.clone(),
                                    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::WatcherBatch,
                                )
                                .for_watcher_session(session.clone()),
                            )),
                        },
                        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::watcher_batch(
                            game_id.clone(),
                            mods_root,
                            changed_paths.clone(),
                            &buffered_events,
                        )
                        .defer_overlay_sync(),
                        watcher_state.inner(),
                        session.clone(),
                    )
                    .await;
                    if let Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result)) = &outcome {
                        disk_reconcile_state.mark_authority_reconciled(
                            &game_id,
                            mods_root,
                            watcher_session,
                            observed_generation,
                            result,
                            &changed_paths,
                        );
                    }
                    outcome
                }
            }
            other => other,
        }
    } else {
        session_recovery
    };
    let acceptance_guard = if activation.is_some() {
        Some(operation_lock.inner_lock().acquire_for_reconcile().await)
    } else {
        None
    };
    let session_recovery = if activation.is_some() {
        match session_recovery {
            Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result))
                if no_pending_disk_commit(operation_lock.inner(), &game_id)
                    && suppressor.pending_repair(&session).is_none()
                    && matches!(
                        disk_reconcile_state.authority_catch_up(&game_id, mods_root, watcher_session),
                        crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Clean {
                            reconcile_revision,
                            ..
                        } if reconcile_revision == result.reconcile_revision
                    ) => Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result)),
            Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(_)) => Err(crate::shared::errors::AppError::Io(
                "Mods changed while indexing finished; retry to verify the disk state".to_string(),
            )),
            other => other,
        }
    } else {
        session_recovery
    };
    match session_recovery {
        Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result))
            if app.state::<WatcherState>().is_current_session(&session) => {
            if let Some(activation) = activation {
                disk_reconcile_state.finish_initial_recovery(
                    &game_id,
                    activation.recovery_generation,
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(
                        Box::new(result.clone()),
                    ),
                );
                settle_pending_onboarding_indexing_after_activation(&app, &pool, &game_id, &result);
            }
            if !emit_reconcile_result_for_current_session(&app, &session, result.clone()) {
                return;
            }
            if let Some(activation) = activation {
                publish_activation_result(&app, &game_id, activation, &result);
            }
            enqueue_telemetry(
                &app,
                [
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Watcher,
                        crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                    ),
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
                        crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                    ),
                ],
            );
        }
        Err(error) if app.state::<WatcherState>().is_current_session(&session) => {
            if let Some(activation) = activation {
                disk_reconcile_state.finish_initial_recovery(
                    &game_id,
                    activation.recovery_generation,
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                        error.to_string(),
                    ),
                );
                emit_activation_failure(&app, &game_id, activation, error.to_string());
            }
            let error_code =
                crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(
                    &error,
                );
            emit_event(
                &app,
                WatchEventPayload::Error {
                    game_id: game_id.clone(),
                    error: error.to_string(),
                    path: Some(mods_path_root.clone()),
                },
            );
            enqueue_telemetry(
                &app,
                [
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Watcher,
                        crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
                        error_code,
                    ),
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
                        crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
                        error_code,
                    ),
                ],
            );
        }
        _ => {
            if let Some(activation) = activation {
                disk_reconcile_state.finish_initial_recovery(
                    &game_id,
                    activation.recovery_generation,
                    crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                        "Activation was superseded".to_string(),
                    ),
                );
            }
            return;
        }
    }

    drop(acceptance_guard);
    drop(activation_claim_guard);
    loop {
        // The debouncer already batches (one callback per debounce window and
        // it sends its whole batch synchronously), so a recv + drain
        // reassembles it without extra timers here.
        let mut batch = Vec::new();
        let Some(first_event) = rx.recv().await else {
            break;
        };
        if !app.state::<WatcherState>().is_current_session(&session) {
            break;
        }
        batch.push(first_event);
        while let Ok(event) = rx.try_recv() {
            batch.push(event);
        }

        log::debug!("Watcher flushing batched events: {}", batch.len());

        // notify emits errors on Windows ReadDirectoryChangesW buffer overflow
        // during mass renames — events were LOST, so a scoped reconcile of the
        // known paths is not enough. Fall back to a full pass.
        let events_lost = batch
            .iter()
            .any(|event| matches!(event, ModWatchEvent::Error(_)));
        if events_lost {
            enqueue_telemetry(
                &app,
                [
                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                        crate::modules::system::application::telemetry::TelemetryOperation::Watcher,
                        crate::modules::system::application::telemetry::TelemetryOutcome::Overflow,
                        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                    ),
                ],
            );
        }
        for event in &batch {
            if let ModWatchEvent::Error(error) = event {
                log::warn!("Watcher error for {}: {}", mods_path_root, error);
            }
        }

        let changed_paths =
            crate::modules::reconciliation::application::disk_reconcile::watcher_batch::collect_changed_paths(&batch);
        let disk_reconcile_state =
            app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let observed_generation = disk_reconcile_state
            .authority_event_generation(&game_id, watcher_session)
            .unwrap_or(0);
        let config = app.state::<crate::modules::settings::application::config::ConfigService>();
        let operation_lock =
            app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
        let context = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: &pool,
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: suppressor.clone(),
            operation_lock: operation_lock.inner_lock(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id.clone(),
                    if events_lost {
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair
                    } else {
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::WatcherBatch
                    },
                )
                .for_watcher_session(session.clone()),
            )),
        };

        // Disk Reconcile only. Watcher must never invoke the Deep Match Scanner pipeline.
        let watcher_state = app.state::<WatcherState>();
        let result = if events_lost {
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_for_watcher(
                context,
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                    game_id.clone(),
                    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                    Vec::new(),
                    true,
                )
                .defer_overlay_sync(),
                watcher_state.inner(),
                session.clone(),
            )
            .await
        } else {
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_from_watcher_batch(
                context,
                game_id.clone(),
                mods_root,
                changed_paths.clone(),
                &batch,
                watcher_state.inner(),
                session.clone(),
            )
            .await
        };

        if !app.state::<WatcherState>().is_current_session(&session) {
            break;
        }
        match result {
            Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result)) => {
                if !disk_reconcile_state.mark_authority_reconciled(
                    &game_id,
                    mods_root,
                    watcher_session,
                    observed_generation,
                    &result,
                    &changed_paths,
                ) {
                    disk_reconcile_state
                        .reject_untrusted_reconcile(&game_id, result.reconcile_revision);
                    continue;
                }
                // The reconcile future returned before this point, releasing
                // its game and operation locks. Queue only the runtime work
                // implied by the committed projection; never await KeyViewer
                // from the watcher loop.
                enqueue_runtime_sync_after_watcher_reconcile(
                    &app,
                    &pool,
                    &game_id,
                    events_lost,
                    &result,
                );
                if !emit_reconcile_result_for_current_session(&app, &session, result) {
                    break;
                }
                enqueue_telemetry(
                    &app,
                    [
                        crate::modules::system::application::telemetry::TelemetryEvent::new(
                            crate::modules::system::application::telemetry::TelemetryOperation::Watcher,
                            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                        ),
                        crate::modules::system::application::telemetry::TelemetryEvent::new(
                            crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
                            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                        ),
                    ],
                );
            }
            Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Superseded) => {
                break;
            }
            Err(error) => {
                let error_code = crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(&error);
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        game_id: game_id.clone(),
                        error: error.to_string(),
                        path: Some(mods_path_root.clone()),
                    },
                );
                enqueue_telemetry(
                    &app,
                    [
                        crate::modules::system::application::telemetry::TelemetryEvent::new(
                            crate::modules::system::application::telemetry::TelemetryOperation::Watcher,
                            crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
                            error_code,
                        ),
                        crate::modules::system::application::telemetry::TelemetryEvent::new(
                            crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
                            crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
                            error_code,
                        ),
                    ],
                );
            }
        }
    }

    app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >()
    .end_authority_session(&game_id, watcher_session, mods_root);
    log::info!("Watcher event loop ended for {}", mods_path_root);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    async fn assert_created_event(
        receiver: &mut crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        expected: &Path,
    ) {
        std::fs::write(expected, "content").expect("write watched file");
        let expected = expected.to_string_lossy().to_string();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match receiver.recv().await {
                    Some(ModWatchEvent::Created(path)) if path == expected => break,
                    Some(_) => continue,
                    None => panic!("installed watcher event channel closed"),
                }
            }
        })
        .await
        .expect("installed watcher should deliver created event");
    }

    fn build_real_watcher(
        state: &WatcherState,
        root: &Path,
        session: WatcherSession,
    ) -> Result<
        (
            crate::modules::workspace::application::scanner::watcher::ModWatcher,
            crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        ),
        ScannerError,
    > {
        crate::modules::workspace::application::scanner::watcher::watch_mod_directory(
            root,
            state.suppressor.clone(),
            session,
        )
    }

    #[test]
    fn activation_reuses_a_fresh_authoritative_reconcile() {
        use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason;

        assert_eq!(
            initial_reconcile_reason(true),
            DiskReconcileReason::GameSwitched
        );
        assert_eq!(
            initial_reconcile_reason(false),
            DiskReconcileReason::ManualRepair
        );
    }

    #[test]
    fn startup_can_install_selected_watcher_but_prewarm_cannot() {
        assert!(install_scope_matches_selection(
            WatcherInstallScope::StartupSelected,
            Some("game-1"),
            "game-1",
        ));
        assert!(!install_scope_matches_selection(
            WatcherInstallScope::InactiveOnly,
            Some("game-1"),
            "game-1",
        ));
        assert!(!install_scope_matches_selection(
            WatcherInstallScope::StartupSelected,
            Some("game-2"),
            "game-1",
        ));
        assert!(install_scope_matches_selection(
            WatcherInstallScope::InactiveOnly,
            Some("game-2"),
            "game-1",
        ));
    }

    #[tokio::test]
    async fn slow_inactive_prewarm_does_not_block_activation() {
        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        let guard = state.activation_guard().await;
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let prewarm = tokio::time::timeout(
            INACTIVE_PREWARM_BUDGET,
            prewarm_without_activation_guard(guard, async move {
                started_tx.send(()).expect("activation waiter");
                std::future::pending::<()>().await;
            }),
        );
        let switch = async {
            started_rx.await.expect("prewarm should start");
            tokio::time::timeout(Duration::from_millis(100), state.activation_guard())
                .await
                .is_ok()
        };
        let (expired, switch_acquired_lease) = tokio::join!(prewarm, switch);
        assert!(expired.is_err());
        assert!(switch_acquired_lease);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn inactive_watcher_takeover_keeps_a_buffered_debounce_event() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let state = WatcherState::new();
        let session = state.prepare_session(root);
        let (watcher, receiver) =
            build_real_watcher(&state, root, session.clone()).expect("install inactive watcher");
        state.install_inactive_watcher(
            "game-1".to_string(),
            root,
            None,
            session,
            watcher,
            receiver,
        );

        let changed = root.join("new-mod.ini");
        std::fs::write(&changed, "content").expect("write before takeover");
        let (adopted_session, watcher, mut receiver) = state
            .take_inactive_watcher_for_handoff("game-1", root, None)
            .expect("take over inactive watcher");
        state.publish_session(&adopted_session);
        *lock(&state.watcher) = Some(watcher);
        let expected = changed.to_string_lossy().to_string();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match receiver.recv().await {
                    Some(ModWatchEvent::Created(path)) if path == expected => break,
                    Some(_) => continue,
                    None => panic!("adopted watcher event channel closed"),
                }
            }
        })
        .await
        .expect("buffered event should survive takeover");
    }

    #[tokio::test]
    async fn clean_active_watcher_survives_inactive_handoff_without_prewarm() {
        use crate::modules::mutation::coordinator::MutationCoordinator;
        use crate::modules::mutation::journal::OperationJournal;
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
            AuthorityCatchUp, DiskReconcileState, InitialRecoveryOutcome, InitialRecoveryReadiness,
        };
        use crate::modules::reconciliation::application::disk_reconcile::types::{
            DiskReconcileReason, DiskReconcileResult, DiskReconcileScanScope, DiskReconcileStatus,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let watcher = WatcherState::new();
        let old_session = watcher.begin_session(&root);
        let (active, _active_receiver) =
            build_real_watcher(&watcher, &root, old_session.clone()).expect("active watcher");
        *lock(&watcher.watcher) = Some(active);
        let state = DiskReconcileState::new();
        state.begin_authority_session("game-1", &root, old_session.generation());
        let mut baseline = DiskReconcileResult {
            game_id: "game-1".to_string(),
            reconcile_revision: 0,
            reason: DiskReconcileReason::StartupBoot,
            status: DiskReconcileStatus::Applied,
            scan_scope: DiskReconcileScanScope::Full,
            folder_conflicts: Vec::new(),
            rename_confirmations: Vec::new(),
            error_message: None,
            changed_roots: Vec::new(),
            objects_changed: false,
            folders_changed: false,
            collections_changed: false,
            runtime_file_changed: false,
            thumbnail_roots: Vec::new(),
            cleared_selection_paths: Vec::new(),
            path_updates: Vec::new(),
            collection_reference_impact: Default::default(),
            change_summary: Default::default(),
            pending_runtime_effects: Default::default(),
            warnings: Vec::new(),
        };
        state.record_result("game-1", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game-1",
            &root,
            old_session.generation(),
            0,
            &baseline,
            &[],
        ));
        let generation = state.mark_initial_recovery_pending("game-1");
        state.finish_initial_recovery(
            "game-1",
            generation,
            InitialRecoveryOutcome::Completed(Box::new(baseline.clone())),
        );
        let session = watcher.prepare_session_with_runtime_config(&root, None);
        let (inactive, receiver) =
            build_real_watcher(&watcher, &root, session.clone()).expect("inactive watcher");
        watcher.install_inactive_watcher(
            "game-1".to_string(),
            &root,
            None,
            session.clone(),
            inactive,
            receiver,
        );
        assert!(prepare_inactive_authority(
            &state,
            "game-1",
            &root,
            Some(old_session.generation()),
            session.generation(),
        ));
        assert!(matches!(
            state.authority_catch_up("game-1", &root, session.generation()),
            AuthorityCatchUp::Clean { .. }
        ));

        assert!(
            tokio::time::timeout(INACTIVE_PREWARM_BUDGET, std::future::pending::<()>())
                .await
                .is_err()
        );
        watcher.invalidate_session();
        *lock(&watcher.watcher) = None;
        let (adopted_session, active, _receiver) = watcher
            .take_inactive_watcher_for_handoff("game-1", &root, None)
            .expect("revisit watcher");
        watcher.publish_session(&adopted_session);
        *lock(&watcher.watcher) = Some(active);
        let journal = Arc::new(
            OperationJournal::open(temp.path().join("journal.json"), 100).expect("journal"),
        );
        let coordinator = MutationCoordinator::with_lock(
            crate::platform::fs::operation_lock::OperationLock::new(),
            journal,
        );
        let (activation, reused) = prepare_activation_recovery(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &root,
            &adopted_session,
            7,
        );
        assert_eq!(activation.recovery_generation, generation);
        assert_eq!(
            reused.expect("clean revisit").scan_scope,
            DiskReconcileScanScope::None
        );
        assert_eq!(
            state.initial_recovery_readiness("game-1"),
            InitialRecoveryReadiness::Ready { generation }
        );

        let mut prewarmed = baseline.clone();
        state.record_result("game-1", &mut prewarmed);
        let observed_generation = state
            .authority_event_generation("game-1", adopted_session.generation())
            .expect("covered session");
        let switching = state.activation_guard().await;
        let acceptance = accept_prewarm_result(
            &state,
            &watcher,
            &coordinator,
            || Some("game-1".to_string()),
            PrewarmAcceptance {
                game_id: "game-1",
                mods_root: &root,
                runtime_config_path: None,
                watcher_session: adopted_session.generation(),
                observed_generation,
                result: &prewarmed,
                changed_paths: &[],
            },
        );
        tokio::pin!(acceptance);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut acceptance)
                .await
                .is_err(),
            "prewarm acceptance must wait for an in-flight switch"
        );
        drop(switching);
        assert!(
            acceptance.await,
            "the adopted watcher still proves coverage"
        );
        assert!(matches!(
            state.authority_catch_up("game-1", &root, adopted_session.generation()),
            AuthorityCatchUp::Clean { reconcile_revision, .. }
                if reconcile_revision == prewarmed.reconcile_revision
        ));
    }

    #[test]
    fn inactive_handoff_keeps_an_event_seen_by_the_new_watcher() {
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
            AuthorityCatchUp, DiskReconcileState,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let state = DiskReconcileState::new();
        state.begin_authority_session("game", &root, 1);
        state.observe_authority_event("game", 2, &root, &[root.join("Changed")], false);
        let observed = state.authority_event_generation("game", 2);

        assert!(!prepare_inactive_authority(
            &state,
            "game",
            &root,
            Some(1),
            2
        ));
        assert_eq!(state.authority_event_generation("game", 2), observed);
        assert!(matches!(
            state.authority_catch_up("game", &root, 2),
            AuthorityCatchUp::Full { .. }
        ));
    }

    #[tokio::test]
    async fn cached_activation_ignores_other_game_lock_but_rejects_target_changes() {
        use crate::modules::mutation::coordinator::MutationCoordinator;
        use crate::modules::mutation::journal::{OperationJournal, OperationPlan, PlannedStep};
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;
        use crate::modules::reconciliation::application::disk_reconcile::types::{
            DiskReconcileReason, DiskReconcileResult, DiskReconcileScanScope, DiskReconcileStatus,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_root).expect("mods root");
        let watcher = WatcherState::new();
        let session = watcher.begin_session(&mods_root);
        let state = DiskReconcileState::new();
        state.begin_authority_session("game-1", &mods_root, session.generation());
        let mut baseline = DiskReconcileResult {
            game_id: "game-1".to_string(),
            reconcile_revision: 0,
            reason: DiskReconcileReason::StartupBoot,
            status: DiskReconcileStatus::Applied,
            scan_scope: DiskReconcileScanScope::Full,
            folder_conflicts: Vec::new(),
            rename_confirmations: Vec::new(),
            error_message: None,
            changed_roots: Vec::new(),
            objects_changed: false,
            folders_changed: false,
            collections_changed: false,
            runtime_file_changed: false,
            thumbnail_roots: Vec::new(),
            cleared_selection_paths: Vec::new(),
            path_updates: Vec::new(),
            collection_reference_impact: Default::default(),
            change_summary: Default::default(),
            pending_runtime_effects: Default::default(),
            warnings: Vec::new(),
        };
        state.record_result("game-1", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game-1",
            &mods_root,
            session.generation(),
            0,
            &baseline,
            &[],
        ));
        let journal = Arc::new(
            OperationJournal::open(temp.path().join("journal.json"), 100).expect("journal"),
        );
        let coordinator = MutationCoordinator::with_lock(
            crate::platform::fs::operation_lock::OperationLock::new(),
            journal.clone(),
        );
        let (startup_cached, changed_paths, force_full, _) = initial_watcher_recovery_plan(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
        );
        assert!(changed_paths.is_empty());
        assert!(
            !force_full,
            "clean startup handoff must not schedule a second scan"
        );
        assert_eq!(
            startup_cached.expect("clean startup projection").scan_scope,
            DiskReconcileScanScope::None
        );
        let ready_generation = state.mark_initial_recovery_pending("game-1");
        state.finish_initial_recovery(
            "game-1",
            ready_generation,
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(
                Box::new(baseline.clone()),
            ),
        );
        let (activation, reused) = prepare_activation_recovery(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
            7,
        );
        assert_eq!(activation.recovery_generation, ready_generation);
        assert_eq!(
            reused.expect("clean activation").scan_scope,
            DiskReconcileScanScope::None
        );
        assert!(matches!(
            state.initial_recovery_readiness("game-1"),
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Ready { generation }
                if generation == ready_generation
        ));
        let cached = || {
            clean_cached_activation_result(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
            )
        };
        assert_eq!(
            cached()
                .expect("clean startup session should reuse projection")
                .scan_scope,
            DiskReconcileScanScope::None
        );

        let guard = coordinator.inner_lock().acquire().await.expect("lock");
        assert!(
            cached().is_some(),
            "another game's operation must not invalidate this proof"
        );
        let (activation, reused) = prepare_activation_recovery(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
            8,
        );
        assert_eq!(
            reused.expect("clean target").scan_scope,
            DiskReconcileScanScope::None
        );
        assert!(matches!(
            state.initial_recovery_readiness("game-1"),
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Ready { generation }
                if generation == activation.recovery_generation && generation == ready_generation
        ));
        drop(guard);

        let target_lease = state
            .acquire_ready_mutation_lease("game-1", coordinator.inner_lock())
            .await
            .expect("target mutation lease");
        assert!(
            cached().is_none(),
            "target game lock must block cache reuse"
        );
        drop(target_lease);

        let old_path = mods_root.join("Old");
        let new_path = mods_root.join("New");
        std::fs::create_dir(&old_path).expect("old mod");
        let id = journal
            .plan_operation(OperationPlan::new(
                "workspace-switch",
                "game-1",
                vec![PlannedStep::rename(0, old_path.clone(), new_path.clone())],
            ))
            .expect("plan");
        journal.mark_applying(&id).expect("applying");
        std::fs::rename(&old_path, &new_path).expect("disk rename");
        journal.mark_step_applied(&id, 0).expect("applied step");
        journal.mark_disk_committed(&id).expect("disk receipt");
        assert!(cached().is_none());
        journal.mark_db_committed(&id).expect("database projection");
        journal.complete(&id).expect("settled operation");

        watcher.suppressor.mark_blanket_event_dropped(&session);
        assert!(cached().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_starts_keep_the_latest_installed_session_delivering_events() {
        let first_dir = tempfile::tempdir().expect("first tempdir");
        let second_dir = tempfile::tempdir().expect("second tempdir");
        let first_root = first_dir.path().to_path_buf();
        let second_root = second_dir.path().to_path_buf();
        let state = Arc::new(WatcherState::new());
        let first_build_started = Arc::new(Barrier::new(2));
        let release_first_build = Arc::new(Barrier::new(2));
        let second_start_attempted = Arc::new(Barrier::new(2));

        let first_handle = {
            let state = state.clone();
            let root = first_root.clone();
            let started = first_build_started.clone();
            let release = release_first_build.clone();
            std::thread::spawn(move || {
                replace_watcher(&state, &root, None, |session| {
                    started.wait();
                    release.wait();
                    build_real_watcher(&state, &root, session)
                })
            })
        };
        first_build_started.wait();

        let second_handle = {
            let state = state.clone();
            let root = second_root.clone();
            let attempted = second_start_attempted.clone();
            std::thread::spawn(move || {
                attempted.wait();
                replace_watcher(&state, &root, None, |session| {
                    build_real_watcher(&state, &root, session)
                })
            })
        };
        second_start_attempted.wait();
        release_first_build.wait();

        let (first_session, _first_receiver, _) = first_handle
            .join()
            .expect("first start thread")
            .expect("first watcher start");
        let (second_session, mut second_receiver, _) = second_handle
            .join()
            .expect("second start thread")
            .expect("second watcher start");

        assert!(!state.is_current_session(&first_session));
        assert!(state.is_current_session(&second_session));
        assert_created_event(&mut second_receiver, &second_root.join("latest.ini")).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_replacement_keeps_the_installed_session_delivering_events() {
        let original_dir = tempfile::tempdir().expect("original tempdir");
        let original_root = original_dir.path().to_path_buf();
        let state = Arc::new(WatcherState::new());
        let (original_session, mut original_receiver, _) =
            replace_watcher(&state, &original_root, None, |session| {
                build_real_watcher(&state, &original_root, session)
            })
            .expect("original watcher start");
        let failed_build_started = Arc::new(Barrier::new(2));
        let release_failed_build = Arc::new(Barrier::new(2));

        let failed_handle = {
            let state = state.clone();
            let started = failed_build_started.clone();
            let release = release_failed_build.clone();
            std::thread::spawn(move || {
                replace_watcher(&state, &PathBuf::from("missing-root"), None, |_session| {
                    started.wait();
                    release.wait();
                    Err(ScannerError::Validation(
                        "injected watcher construction failure".to_string(),
                    ))
                })
            })
        };
        failed_build_started.wait();
        release_failed_build.wait();
        let error = match failed_handle.join().expect("failed start thread") {
            Ok(_) => panic!("replacement should fail"),
            Err(error) => error,
        };

        assert!(matches!(error, ScannerError::Validation(_)));
        assert!(state.is_current_session(&original_session));
        assert_created_event(
            &mut original_receiver,
            &original_root.join("still-current.ini"),
        )
        .await;
    }
}
