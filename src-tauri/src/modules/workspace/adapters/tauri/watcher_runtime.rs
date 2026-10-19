//! Tauri watcher orchestration and the single receiver task.

use crate::modules::workspace::application::scanner::watcher::lifecycle::{
    accept_prewarm_result, activation_publication_allowed, activation_result_is_accepted,
    clean_cached_activation_result, initial_activation_waits_for_barrier, initial_reconcile_reason,
    initial_watcher_recovery_plan, install_scope_matches_selection, no_pending_disk_commit,
    pending_activation_catch_up_plan, pending_watcher_catch_up_plan, prepare_activation_recovery,
    prepare_inactive_authority, retire_superseded_activation, try_clean_activation_recovery_result,
    PrewarmAcceptance, WatcherAuthorityContext, WatcherCatchUpPlan, WatcherInstallScope,
};
pub(crate) use crate::modules::workspace::application::scanner::watcher::lifecycle::{
    authority_catch_up_backoff, pending_activation_target, prepare_current_activation_reuse,
    watcher_identity_cache_failure, WatcherActivation, WATCHER_IDENTITY_COVERAGE_ERROR,
};
use crate::modules::workspace::application::scanner::watcher::{
    ModWatchEvent, ModWatcher, WatchEventPayload, WatchEventReceiver, WatcherSession, WatcherState,
    WatcherSuppressor,
};
use crate::shared::errors::ScannerError;
use crate::shared::sync::lock;
use std::sync::Arc;
use tauri::{Emitter, Manager};

const INACTIVE_PREWARM_BUDGET: std::time::Duration = std::time::Duration::from_millis(250);

struct WatcherLoopContext {
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
    mods_path_root: String,
    suppressor: Arc<WatcherSuppressor>,
    session: WatcherSession,
    activation: Option<WatcherActivation>,
}

struct WatcherReceiverLifetime<'a> {
    state:
        &'a crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &'a str,
    root: &'a std::path::Path,
    session: u64,
    activation: Option<WatcherActivation>,
    on_end: Option<Box<dyn Fn() + Send + Sync + 'a>>,
}

impl Drop for WatcherReceiverLifetime<'_> {
    fn drop(&mut self) {
        self.state
            .end_authority_session(self.game_id, self.session, self.root);
        if let Some(activation) = self.activation {
            self.state.finish_initial_recovery(
                self.game_id, activation.recovery_generation,
                crate::modules::reconciliation::api::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                    "Watcher receiver ended before recovery completed; retry activation".into(),
                ),
            );
        }
        if let Some(on_end) = &self.on_end {
            on_end();
        }
    }
}

fn emit_event(app: &tauri::AppHandle, payload: WatchEventPayload) {
    let _ = app.emit("mod_watch:event", payload);
}

pub(crate) fn inactive_activation_is_ready(
    app: &tauri::AppHandle,
    watcher: &WatcherState,
    game_id: &str,
    mods_root: &std::path::Path,
    runtime_config_path: &std::path::Path,
) -> bool {
    let Some(session) =
        watcher.inactive_session_for_coverage(game_id, mods_root, Some(runtime_config_path))
    else {
        return false;
    };
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

async fn prewarm_without_activation_guard<T>(
    guard: tokio::sync::MutexGuard<'_, ()>,
    work: impl std::future::Future<Output = T>,
) -> T {
    drop(guard);
    work.await
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

    let generation = if events_lost || result.changed_roots.is_empty() {
        crate::modules::reconciliation::api::enqueue_runtime_sync(
            app,
            pool,
            game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
        )
    } else {
        crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
            app,
            pool,
            game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
            crate::modules::reconciliation::api::runtime_sync_request_for_roots(
                &result.changed_roots,
            ),
        )
    };
    #[cfg(feature = "watcher-verification")]
    super::watcher_verification::on_runtime_enqueue(
        app,
        game_id,
        result.reconcile_revision,
        generation,
    );
    #[cfg(not(feature = "watcher-verification"))]
    let _ = generation;
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
    let deferred_app = app.clone();
    let deferred_game_id = game_id.clone();
    let deferred_root = mods_root.clone();
    let deferred_session = session.clone();
    #[cfg(debug_assertions)]
    let observation_samples = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    #[cfg(debug_assertions)]
    let deferred_samples = observation_samples.clone();
    let on_unproven = Arc::new(move |paths: &[std::path::PathBuf], events_lost| {
        #[cfg(debug_assertions)]
        if deferred_samples.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 16 {
            log::info!("watcher authority unproven game_id={} session={} source=deferred_echo path_count={} coverage_lost={}", deferred_game_id, watcher_session, paths.len(), events_lost);
        }
        let state = deferred_app.state::<
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
        >();
        state.observe_authority_event(
            &deferred_game_id,
            watcher_session,
            &deferred_root,
            paths,
            events_lost,
        );
        deferred_session.request_work();
    });
    std::sync::Arc::new(move |paths, kind, events_lost| {
        let state = app.state::<
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
        >();
        if events_lost {
            suppressor.invalidate_rename_echoes(&session);
        }
        if !events_lost
            && kind.is_some_and(|kind| {
                suppressor.observe_expected_rename_echo(
                    &game_id,
                    &session,
                    kind,
                    paths,
                    on_unproven.clone(),
                )
            })
        {
            return;
        }
        #[cfg(debug_assertions)]
        if observation_samples.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 16 {
            let directory_count = paths.iter().filter(|path| path.is_dir()).count();
            let relative_depths = paths
                .iter()
                .map(|path| {
                    crate::shared::path_key::physical_namespace_path(path)
                        .ok()
                        .and_then(|path| {
                            let root = crate::shared::path_key::physical_namespace_path(&mods_root)
                                .ok()?;
                            Some(path.strip_prefix(root).ok()?.components().count())
                        })
                })
                .collect::<Vec<_>>();
            log::info!("watcher authority unproven game_id={} session={} source=event kind={:?} path_count={} directory_count={} relative_depths={:?} coverage_lost={}", game_id, watcher_session, kind, paths.len(), directory_count, relative_depths, events_lost);
        }
        state.observe_authority_event(&game_id, watcher_session, &mods_root, paths, events_lost);
    })
}

pub(crate) async fn start_inactive_watcher(
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
    .await
}

pub(crate) async fn start_inactive_watcher_with_activation_guard(
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
    let covered_session = {
        let active_watcher = lock(&state.watcher);
        active_watcher
            .as_ref()
            .and_then(|_| state.current_session_for_coverage(mods_root, runtime_config_path))
            .map(|session| session.generation())
    };
    let session = state.prepare_session_with_runtime_config(mods_root, runtime_config_path);
    let observer = authority_observer(app, game_id, mods_root, &session, state.suppressor.clone());
    let watcher_result =
        crate::modules::workspace::application::scanner::watcher::register_watcher_candidate(
            mods_root.to_path_buf(),
            runtime_config_path.map(std::path::Path::to_path_buf),
            state.suppressor.clone(),
            session.clone(),
            observer,
        )
        .await;
    let candidate = match watcher_result {
        Ok(installed) => installed,
        Err(error) => {
            app.state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >()
            .invalidate_authority(game_id, mods_root);
            return Err(error);
        }
    };
    if !state.registration_generation_is_current(candidate.session())
        || !watcher_install_configuration_matches(
            app,
            game_id,
            mods_root,
            runtime_config_path,
            scope,
        )
    {
        return Ok(None);
    }
    let (session, watcher, receiver) = candidate.accept(mods_root, runtime_config_path)?;
    if let Some(error) = watcher_identity_cache_failure(&session) {
        return Err(ScannerError::Validation(error.into()));
    }
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
        &session,
    );
    Ok(Some(session.generation()))
}

fn watcher_install_configuration_matches(
    app: &tauri::AppHandle,
    game_id: &str,
    root: &std::path::Path,
    runtime_config_path: Option<&std::path::Path>,
    scope: WatcherInstallScope,
) -> bool {
    let settings = app
        .state::<crate::modules::settings::api::config::ConfigService>()
        .get_settings();
    install_scope_matches_selection(scope, settings.active_game_id.as_deref(), game_id)
        && settings
            .games
            .iter()
            .find(|game| game.id == game_id)
            .is_some_and(|game| {
                crate::shared::path_key::exact_location_key_for_path(&game.mod_path)
                    == crate::shared::path_key::exact_location_key_for_path(root)
                    && runtime_config_path.is_none_or(|runtime| {
                        crate::shared::path_key::exact_location_key_for_path(
                            &game.instance_path.join("d3dx.ini"),
                        ) == crate::shared::path_key::exact_location_key_for_path(runtime)
                    })
            })
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
            )
            .await
            {
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

fn emit_activation_result_with_current_authority(
    app: &tauri::AppHandle,
    game_id: &str,
    activation: WatcherActivation,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    authority: crate::modules::reconciliation::application::disk_reconcile::orchestrator::ActivationAuthority,
) {
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        DiskReconcileStatus, GameActivationPhase, GameActivationStatus,
    };

    let phase = match result.status {
        DiskReconcileStatus::Applied | DiskReconcileStatus::AppliedWithFolderConflicts => {
            GameActivationPhase::Ready
        }
        DiskReconcileStatus::SourceUnavailable => GameActivationPhase::SourceUnavailable,
        DiskReconcileStatus::NeedsRenameConfirmation => GameActivationPhase::Failed,
    };
    let is_ready = matches!(phase, GameActivationPhase::Ready);
    let runtime_sync_generation = is_ready.then(|| {
        crate::modules::reconciliation::api::enqueue_runtime_sync_with_authority(
            app,
            app.state::<sqlx::SqlitePool>().inner(),
            game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::GameActivated,
            authority,
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
}

pub(crate) enum RecoveredActivationPublication {
    Published,
    Retry,
    Superseded,
}

pub(crate) fn publish_activation_outcome(
    app: &tauri::AppHandle,
    game_id: &str,
    session: &WatcherSession,
    suppressor: &WatcherSuppressor,
    mods_root: &std::path::Path,
    activation: WatcherActivation,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) -> RecoveredActivationPublication {
    if !app.state::<WatcherState>().is_current_session(session) {
        return RecoveredActivationPublication::Superseded;
    }
    let state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    if !state.initial_recovery_generation_is_current(game_id, activation.recovery_generation) {
        return RecoveredActivationPublication::Superseded;
    }
    let Some(authority) =
        state.activation_authority_for_generation(game_id, activation.activation_generation)
    else {
        return RecoveredActivationPublication::Superseded;
    };
    let coordinator = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
    let pool = app.state::<sqlx::SqlitePool>();
    authority.with_current(|| {
        if !state.initial_recovery_generation_is_current(game_id, activation.recovery_generation) {
            return RecoveredActivationPublication::Superseded;
        }
        let validate_ready = || activation_result_is_accepted(
            state.inner(),
            suppressor,
            coordinator.inner(),
            game_id,
            mods_root,
            session,
            result,
        );
        if !activation_publication_allowed(result, validate_ready) {
            return RecoveredActivationPublication::Retry;
        }
        if !emit_reconcile_result_for_current_session(app, session, result.clone()) {
            return RecoveredActivationPublication::Superseded;
        }
        if !activation_publication_allowed(result, validate_ready) {
            return RecoveredActivationPublication::Retry;
        }
        if !state.initial_recovery_generation_is_current(game_id, activation.recovery_generation) {
            return RecoveredActivationPublication::Superseded;
        }
        state.finish_initial_recovery(
            game_id,
            activation.recovery_generation,
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(
                Box::new(result.clone()),
            ),
        );
        settle_pending_onboarding_indexing_after_activation(app, pool.inner(), game_id, result);
        emit_activation_result_with_current_authority(
            app, game_id, activation, result, authority.clone(),
        );
        RecoveredActivationPublication::Published
    }).unwrap_or(RecoveredActivationPublication::Superseded)
}

pub(crate) fn emit_activation_failure(
    app: &tauri::AppHandle,
    game_id: &str,
    activation: WatcherActivation,
    error: String,
) {
    let unavailable = app
        .state::<crate::modules::settings::api::config::ConfigService>()
        .get_settings()
        .games
        .iter()
        .find(|game| game.id == game_id)
        .is_some_and(|game| !game.mod_path.is_dir());
    let phase = if unavailable {
        crate::modules::reconciliation::api::disk_reconcile::types::GameActivationPhase::SourceUnavailable
    } else {
        crate::modules::reconciliation::api::disk_reconcile::types::GameActivationPhase::Failed
    };
    emit_activation_failure_with_phase(app, game_id, activation, phase, error);
}

pub(crate) fn emit_activation_failure_with_phase(
    app: &tauri::AppHandle,
    game_id: &str,
    activation: WatcherActivation,
    phase: crate::modules::reconciliation::api::disk_reconcile::types::GameActivationPhase,
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
            phase,
            reconcile_revision: None,
            runtime_sync_generation: None,
            error: Some(error),
        };
        if let Err(error) = app.emit("game_activation:status", status) {
            log::warn!("Could not emit game activation failure: {error}");
        }
    });
}

fn publish_registered_watcher(
    state: &WatcherState,
    active_watcher: &mut Option<ModWatcher>,
    session: &WatcherSession,
    watcher: ModWatcher,
) -> Result<(), ScannerError> {
    if !state.publish_registered_session(session) {
        return Err(ScannerError::Validation(
            "Watcher session ended during registration; retry activation".into(),
        ));
    }
    *active_watcher = Some(watcher);
    Ok(())
}

#[cfg(test)]
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
    if let Some(error) = watcher_identity_cache_failure(&session) {
        return Err(ScannerError::Validation(error.into()));
    }
    if active_watcher.is_some() {
        log::info!("Stopping existing watcher");
    }
    publish_registered_watcher(state, &mut active_watcher, &session, watcher)?;
    Ok((session, receiver, covered_session))
}

pub(crate) async fn start_watcher(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
) -> Result<(), ScannerError> {
    start_watcher_inner(app, state, pool, path, game_id, None, None)
        .await
        .map(|_| ())
}

pub(crate) async fn start_watcher_for_activation(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
    activation_generation: u64,
    activation_intent: u64,
) -> Result<
    Option<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult>,
    ScannerError,
> {
    start_watcher_inner(
        app,
        state,
        pool,
        path,
        game_id,
        Some(activation_generation),
        Some(activation_intent),
    )
    .await
}

async fn start_watcher_inner(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
    activation_generation: Option<u64>,
    activation_intent: Option<u64>,
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
    let admission_state = app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>();
    if activation_generation.is_some_and(|generation| {
        !admission_state.activation_is_current(Some(&game_id), generation)
    }) || activation_intent
        .is_some_and(|intent| !admission_state.activation_intent_is_current(intent))
        || !watcher_install_configuration_matches(
            &app,
            &game_id,
            path_obj,
            runtime_config_path.as_deref(),
            WatcherInstallScope::StartupSelected,
        )
    {
        return Err(ScannerError::Validation(
            "Watcher registration was superseded; retry the current selection".into(),
        ));
    }
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
        let session =
            state.prepare_session_with_runtime_config(path_obj, runtime_config_path.as_deref());
        let observer =
            authority_observer(&app, &game_id, path_obj, &session, state.suppressor.clone());
        let candidate = match crate::modules::workspace::application::scanner::watcher::register_watcher_candidate(
            path_obj.to_path_buf(),
            runtime_config_path.clone(),
            state.suppressor.clone(),
            session.clone(),
            observer,
        )
        .await
        {
            Ok(candidate) => candidate,
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
        if !state.registration_generation_is_current(candidate.session())
            || activation_generation.is_some_and(|generation| {
                !disk_reconcile_state.activation_is_current(Some(&game_id), generation)
            })
            || activation_intent
                .is_some_and(|intent| !disk_reconcile_state.activation_intent_is_current(intent))
            || !watcher_install_configuration_matches(
                &app,
                &game_id,
                path_obj,
                runtime_config_path.as_deref(),
                WatcherInstallScope::StartupSelected,
            )
        {
            return Err(ScannerError::Validation(
                "Watcher registration was superseded; retry the current selection".into(),
            ));
        }
        let (session, watcher, receiver) =
            candidate.accept(path_obj, runtime_config_path.as_deref())?;
        {
            let mut active_watcher = lock(&state.watcher);
            publish_registered_watcher(state, &mut active_watcher, &session, watcher)?;
        }
        let _ = state.take_inactive_watcher_for_handoff(
            &game_id,
            path_obj,
            runtime_config_path.as_deref(),
        );
        disk_reconcile_state.begin_authority_session_with_coverage(&game_id, path_obj, &session);
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

enum WatcherWaitOutcome {
    Event(ModWatchEvent),
    Work,
    Deadline,
    Closed,
}

async fn wait_for_watcher_activity(
    receiver: &mut WatchEventReceiver,
    session: &WatcherSession,
    retry_at: Option<tokio::time::Instant>,
) -> WatcherWaitOutcome {
    let deadline = async {
        match retry_at {
            Some(at) => tokio::time::sleep_until(at).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        event = receiver.recv() => match event {
            Some(event) => WatcherWaitOutcome::Event(event),
            None => WatcherWaitOutcome::Closed,
        },
        () = session.wait_for_work() => WatcherWaitOutcome::Work,
        () = deadline => WatcherWaitOutcome::Deadline,
    }
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
        mut activation,
    } = context;
    let disk_reconcile_state =
        app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>();
    let mods_root = std::path::Path::new(&mods_path_root);
    let watcher_session = session.generation();
    let receiver_activation = activation;
    let _receiver_lifetime = WatcherReceiverLifetime {
        state: disk_reconcile_state.inner(),
        game_id: &game_id,
        root: mods_root,
        session: watcher_session,
        activation,
        on_end: Some(Box::new(|| {
            if app
                .try_state::<WatcherState>()
                .is_some_and(|watcher| watcher.is_current_session(&session))
            {
                const ERROR: &str =
                    "Watcher receiver ended; retry activation to restore live coverage";
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        game_id: game_id.clone(),
                        error: ERROR.into(),
                        path: None,
                    },
                );
                if let Some(duty) = receiver_activation {
                    emit_activation_failure(&app, &game_id, duty, ERROR.into());
                }
            }
        })),
    };
    if !app.state::<WatcherState>().is_current_session(&session) {
        return;
    }
    // Startup and activation can both inherit continuous watcher coverage.
    let config = app.state::<crate::modules::settings::application::config::ConfigService>();
    let operation_lock = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
    let watcher_state = app.state::<WatcherState>();
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
    let session_recovery = if let Some(error) = watcher_identity_cache_failure(&session) {
        Err(crate::shared::errors::AppError::Io(error.into()))
    } else if let Some(result) = cached_result.or(joined_background_result) {
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
        let snapshot_requires_repair = if matches!(&outcome, Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(_))) {
            if let Some(lease) = &onboarding_lease {
                crate::modules::workspace::application::scanner::watcher::onboarding_snapshot_requires_repair(lease, operation_lock.inner(), &game_id).await
            } else {
                false
            }
        } else {
            false
        };
        if snapshot_requires_repair {
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
    retire_superseded_activation(disk_reconcile_state.inner(), &game_id, &mut activation);
    let session_recovery = if rx.is_closed() {
        Err(crate::shared::errors::AppError::Io(
            "Watcher receiver ended during recovery; retry activation".into(),
        ))
    } else {
        session_recovery
    };
    let acceptance_guard = if activation.is_some() {
        Some(operation_lock.inner_lock().acquire_for_reconcile().await)
    } else {
        None
    };
    let initial_recovery_failed = session_recovery.is_err();
    let mut initial_authority_pending = !matches!(
        &session_recovery,
        Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result))
            if activation_result_is_accepted(
                disk_reconcile_state.inner(),
                suppressor.as_ref(),
                operation_lock.inner(),
                &game_id,
                mods_root,
                &session,
                result,
            )
    );
    match session_recovery {
        Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result))
            if app.state::<WatcherState>().is_current_session(&session) => {
            if initial_activation_waits_for_barrier(activation.is_some(), initial_authority_pending, &result) {
                log::debug!("Activation is waiting for the current disk and watcher authority barriers");
            } else if let Some(duty) = activation {
                match publish_activation_outcome(
                    &app,
                    &game_id,
                    &session,
                    suppressor.as_ref(),
                    mods_root,
                    duty,
                    &result,
                ) {
                    RecoveredActivationPublication::Published => {}
                    RecoveredActivationPublication::Retry => {
                        initial_authority_pending = true;
                        log::debug!("Activation authority changed before Ready; scheduling catch-up");
                    }
                    RecoveredActivationPublication::Superseded => {
                        activation = None;
                        if !watcher_state.is_current_session(&session) {
                            return;
                        }
                    }
                }
            } else if !emit_reconcile_result_for_current_session(&app, &session, result.clone()) {
                return;
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
            if !watcher_state.is_current_session(&session) {
                return;
            }
        }
    }

    drop(acceptance_guard);
    drop(activation_claim_guard);
    if watcher_identity_cache_failure(&session).is_some() {
        activation = None;
    }
    // A rejected projection means the disk may have changed after its input
    // snapshot. Retry from the authority's *current* dirty set. Back off after
    // an immediate pair, so an unavailable source cannot spin or block a
    // foreground disk-first switch.
    const MAX_CONSECUTIVE_AUTHORITY_CATCH_UPS: usize = 2;
    let mut pending_catch_up = initial_authority_pending
        .then(|| {
            pending_activation_catch_up_plan(
                disk_reconcile_state.inner(),
                suppressor.as_ref(),
                &game_id,
                mods_root,
                &session,
            )
        })
        .flatten();
    if initial_recovery_failed && pending_catch_up.is_none() {
        pending_catch_up = Some(WatcherCatchUpPlan {
            changed_paths: Vec::new(),
            force_full: true,
            observed_generation: disk_reconcile_state
                .authority_event_generation(&game_id, watcher_session)
                .unwrap_or(0),
        });
    }
    let mut retry_after =
        (activation.is_some() && initial_authority_pending && pending_catch_up.is_none())
            .then(|| tokio::time::Instant::now() + authority_catch_up_backoff(0));
    let mut consecutive_catch_ups = 0;
    let mut authority_retry_round = 0;
    let mut activation_recovery_pending = activation.is_some() && initial_authority_pending;
    loop {
        let disk_reconcile_state =
            app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        if !watcher_state.is_current_session(&session) {
            break;
        }
        let authority_context = WatcherAuthorityContext {
            state: disk_reconcile_state.inner(),
            suppressor: suppressor.as_ref(),
            game_id: &game_id,
            mods_root,
            session: &session,
        };
        authority_context.retire_publication(
            &mut activation,
            &mut activation_recovery_pending,
            &mut pending_catch_up,
            &mut retry_after,
        );
        let (batch, changed_paths, events_lost, force_full, observed_generation, is_retry) =
            if let Some(plan) = pending_catch_up.take() {
                log::debug!("Watcher retrying disk authority catch-up");
                (
                    Vec::new(),
                    plan.changed_paths,
                    false,
                    plan.force_full,
                    plan.observed_generation,
                    true,
                )
            } else {
                // The debouncer already batches (one callback per debounce window and
                // it sends its whole batch synchronously), so a recv + drain
                // reassembles it without extra timers here.
                let retry_at = retry_after.take();
                #[cfg(feature = "watcher-verification")]
                super::watcher_verification::on_wait(&app, &game_id, mods_root, &session, retry_at);
                let outcome = wait_for_watcher_activity(&mut rx, &session, retry_at).await;
                #[cfg(feature = "watcher-verification")]
                super::watcher_verification::on_wake(
                    &app,
                    &session,
                    matches!(outcome, WatcherWaitOutcome::Work),
                );
                let first_event = match outcome {
                    WatcherWaitOutcome::Event(event) => event,
                    WatcherWaitOutcome::Closed => break,
                    WatcherWaitOutcome::Work => {
                        pending_catch_up = pending_activation_catch_up_plan(
                            disk_reconcile_state.inner(),
                            suppressor.as_ref(),
                            &game_id,
                            mods_root,
                            &session,
                        );
                        if pending_catch_up.is_none() {
                            retry_after = retry_at;
                        }
                        continue;
                    }
                    WatcherWaitOutcome::Deadline => {
                        if !app.state::<WatcherState>().is_current_session(&session) {
                            break;
                        }
                        authority_context.retire_publication(
                            &mut activation,
                            &mut activation_recovery_pending,
                            &mut pending_catch_up,
                            &mut retry_after,
                        );
                        if let Some(plan) = pending_activation_catch_up_plan(
                            disk_reconcile_state.inner(),
                            suppressor.as_ref(),
                            &game_id,
                            mods_root,
                            &session,
                        ) {
                            consecutive_catch_ups = 0;
                            pending_catch_up = Some(plan);
                        } else if activation_recovery_pending {
                            let coordinator = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
                            let recovered = activation
                                .filter(|activation| {
                                    disk_reconcile_state.activation_is_current(
                                        Some(&game_id),
                                        activation.activation_generation,
                                    )
                                })
                                .and_then(|activation| {
                                    try_clean_activation_recovery_result(
                                        disk_reconcile_state.inner(),
                                        suppressor.as_ref(),
                                        coordinator.inner(),
                                        &game_id,
                                        mods_root,
                                        &session,
                                    )
                                    .map(|(result, guard)| (activation, result, guard))
                                });
                            if let Some((duty, result, _guard)) = recovered {
                                match publish_activation_outcome(
                                    &app,
                                    &game_id,
                                    &session,
                                    suppressor.as_ref(),
                                    mods_root,
                                    duty,
                                    &result,
                                ) {
                                    RecoveredActivationPublication::Published => {
                                        activation_recovery_pending = false;
                                    }
                                    RecoveredActivationPublication::Retry => {
                                        if let Some(plan) = pending_activation_catch_up_plan(
                                            disk_reconcile_state.inner(),
                                            suppressor.as_ref(),
                                            &game_id,
                                            mods_root,
                                            &session,
                                        ) {
                                            pending_catch_up = Some(plan);
                                        } else {
                                            retry_after = Some(
                                                tokio::time::Instant::now()
                                                    + authority_catch_up_backoff(
                                                        authority_retry_round,
                                                    ),
                                            );
                                            authority_retry_round =
                                                authority_retry_round.saturating_add(1);
                                        }
                                    }
                                    RecoveredActivationPublication::Superseded => {
                                        authority_context.retire_publication(
                                            &mut activation,
                                            &mut activation_recovery_pending,
                                            &mut pending_catch_up,
                                            &mut retry_after,
                                        );
                                        if !watcher_state.is_current_session(&session) {
                                            break;
                                        }
                                    }
                                }
                            } else {
                                retry_after = Some(
                                    tokio::time::Instant::now()
                                        + authority_catch_up_backoff(authority_retry_round),
                                );
                                authority_retry_round = authority_retry_round.saturating_add(1);
                            }
                        }
                        continue;
                    }
                };
                if !app.state::<WatcherState>().is_current_session(&session) {
                    break;
                }
                let mut batch = vec![first_event];
                while let Ok(event) = rx.try_recv() {
                    batch.push(event);
                }

                log::debug!("Watcher flushing batched events: {}", batch.len());
                consecutive_catch_ups = 0;
                authority_retry_round = 0;

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
                let observed_generation = disk_reconcile_state
                    .authority_event_generation(&game_id, watcher_session)
                    .unwrap_or(0);
                (
                    batch,
                    changed_paths,
                    events_lost,
                    false,
                    observed_generation,
                    false,
                )
            };
        if let Some(error) = watcher_identity_cache_failure(&session) {
            let failed_activation = activation.take();
            if let Some(duty) = failed_activation {
                disk_reconcile_state.finish_initial_recovery(&game_id, duty.recovery_generation, crate::modules::reconciliation::api::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(error.into()));
                emit_activation_failure(&app, &game_id, duty, error.into());
                activation_recovery_pending = false;
            }
            if !is_retry || failed_activation.is_some() {
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        game_id: game_id.clone(),
                        error: error.into(),
                        path: Some(mods_path_root.clone()),
                    },
                );
            }
            retry_after = Some(
                tokio::time::Instant::now() + authority_catch_up_backoff(authority_retry_round),
            );
            authority_retry_round = authority_retry_round.saturating_add(1);
            continue;
        }
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
                    if events_lost || force_full {
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair
                    } else {
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::WatcherBatch
                    },
                )
                .for_watcher_session(session.clone()),
            )),
        };

        // Disk Reconcile only. Watcher must never invoke the Deep Match Scanner pipeline.
        let request = if is_retry || events_lost || force_full {
            crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                    game_id.clone(),
                    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                    changed_paths.clone(),
                    events_lost || force_full,
                )
                .defer_overlay_sync()
        } else {
            crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileRequest::watcher_batch(
                game_id.clone(), mods_root, changed_paths.clone(), &batch,
            )
        };
        // Ordinary batches use the same non-blocking path as catch-up. Dirty
        // authority survives a deferred/cancelled scan and drives the timer retry.
        let result = match crate::modules::reconciliation::api::disk_reconcile::orchestrator::try_reconcile_disk_state_for_prewarm(
            context, request.for_watcher_session(session.clone()),
        ).await {
                Ok(Some(result)) => Ok(result),
                Ok(None) => {
                    retry_after = Some(
                        tokio::time::Instant::now()
                            + authority_catch_up_backoff(authority_retry_round),
                    );
                    authority_retry_round = authority_retry_round.saturating_add(1);
                    continue;
                }
                Err(error) => Err(error),
        };

        if !app.state::<WatcherState>().is_current_session(&session) {
            break;
        }
        match result {
            Ok(result) => {
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
                    if consecutive_catch_ups < MAX_CONSECUTIVE_AUTHORITY_CATCH_UPS {
                        if let Some(plan) = pending_watcher_catch_up_plan(
                            disk_reconcile_state.inner(),
                            &game_id,
                            mods_root,
                            watcher_session,
                        ) {
                            consecutive_catch_ups += 1;
                            pending_catch_up = Some(plan);
                            continue;
                        }
                    }
                    let error = "Watcher reconcile could not establish current disk authority; retrying from the current filesystem state";
                    log::error!("{error} for {mods_path_root}");
                    emit_event(
                        &app,
                        WatchEventPayload::Error {
                            game_id: game_id.clone(),
                            error: error.to_string(),
                            path: Some(mods_path_root.clone()),
                        },
                    );
                    if pending_watcher_catch_up_plan(
                        disk_reconcile_state.inner(),
                        &game_id,
                        mods_root,
                        watcher_session,
                    )
                    .is_some()
                    {
                        retry_after = Some(
                            tokio::time::Instant::now()
                                + authority_catch_up_backoff(authority_retry_round),
                        );
                        authority_retry_round = authority_retry_round.saturating_add(1);
                    }
                    continue;
                }
                authority_context.retire_publication(
                    &mut activation,
                    &mut activation_recovery_pending,
                    &mut pending_catch_up,
                    &mut retry_after,
                );
                let recovering_activation = activation_recovery_pending
                    && activation.is_some_and(|activation| {
                        disk_reconcile_state
                            .activation_is_current(Some(&game_id), activation.activation_generation)
                    });
                if recovering_activation {
                    let coordinator =
                        app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
                    if let Some((accepted, _guard)) = try_clean_activation_recovery_result(
                        disk_reconcile_state.inner(),
                        suppressor.as_ref(),
                        coordinator.inner(),
                        &game_id,
                        mods_root,
                        &session,
                    ) {
                        match publish_activation_outcome(
                            &app,
                            &game_id,
                            &session,
                            suppressor.as_ref(),
                            mods_root,
                            activation.expect("current activation"),
                            &accepted,
                        ) {
                            RecoveredActivationPublication::Published => {
                                activation_recovery_pending = false;
                            }
                            RecoveredActivationPublication::Retry => {
                                if let Some(plan) = pending_activation_catch_up_plan(
                                    disk_reconcile_state.inner(),
                                    suppressor.as_ref(),
                                    &game_id,
                                    mods_root,
                                    &session,
                                ) {
                                    pending_catch_up = Some(plan);
                                } else {
                                    retry_after = Some(
                                        tokio::time::Instant::now()
                                            + authority_catch_up_backoff(authority_retry_round),
                                    );
                                    authority_retry_round = authority_retry_round.saturating_add(1);
                                }
                                continue;
                            }
                            RecoveredActivationPublication::Superseded => {
                                authority_context.retire_publication(
                                    &mut activation,
                                    &mut activation_recovery_pending,
                                    &mut pending_catch_up,
                                    &mut retry_after,
                                );
                                if !watcher_state.is_current_session(&session) {
                                    break;
                                }
                            }
                        }
                    } else {
                        retry_after = Some(
                            tokio::time::Instant::now()
                                + authority_catch_up_backoff(authority_retry_round),
                        );
                        authority_retry_round = authority_retry_round.saturating_add(1);
                        continue;
                    }
                } else {
                    // The reconcile future returned before this point, releasing
                    // its game and operation locks. Queue only the runtime work
                    // implied by the committed projection; never await KeyViewer
                    // from the watcher loop.
                    #[cfg(feature = "watcher-verification")]
                    super::watcher_verification::on_publication(&app, &session, &result);
                    enqueue_runtime_sync_after_watcher_reconcile(
                        &app,
                        &pool,
                        &game_id,
                        events_lost || force_full,
                        &result,
                    );
                    if !emit_reconcile_result_for_current_session(&app, &session, result.clone()) {
                        break;
                    }
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
                if consecutive_catch_ups < MAX_CONSECUTIVE_AUTHORITY_CATCH_UPS {
                    if let Some(plan) = pending_watcher_catch_up_plan(
                        disk_reconcile_state.inner(),
                        &game_id,
                        mods_root,
                        watcher_session,
                    ) {
                        consecutive_catch_ups += 1;
                        pending_catch_up = Some(plan);
                    }
                } else if pending_watcher_catch_up_plan(
                    disk_reconcile_state.inner(),
                    &game_id,
                    mods_root,
                    watcher_session,
                )
                .is_some()
                {
                    retry_after = Some(
                        tokio::time::Instant::now()
                            + authority_catch_up_backoff(authority_retry_round),
                    );
                    authority_retry_round = authority_retry_round.saturating_add(1);
                }
            }
        }
    }

    log::info!("Watcher event loop ended for {}", mods_path_root);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    #[tokio::test]
    async fn canceled_idle_and_backoff_waits_leave_the_receiver_and_authority_live() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState;

        for backoff in [false, true] {
            let root = tempfile::tempdir().expect("root");
            let watcher = WatcherState::new();
            let session = watcher.begin_session(root.path());
            let state = DiskReconcileState::new();
            state.begin_authority_session_with_coverage("game", root.path(), &session);
            let (sender, mut receiver) = WatchEventReceiver::test_channel();
            let retry_at = backoff.then(|| tokio::time::Instant::now() + Duration::from_secs(30));
            assert!(tokio::time::timeout(
                Duration::from_millis(10),
                wait_for_watcher_activity(&mut receiver, &session, retry_at),
            )
            .await
            .is_err());
            assert!(watcher.is_current_session(&session));
            assert!(watcher_identity_cache_failure(&session).is_none());
            assert_eq!(
                state.authority_event_generation("game", session.generation()),
                Some(0)
            );

            sender
                .send(ModWatchEvent::Modified("after-cancellation.ini".into()))
                .await
                .expect("receiver remains live");
            assert!(matches!(
                wait_for_watcher_activity(&mut receiver, &session, retry_at).await,
                WatcherWaitOutcome::Event(ModWatchEvent::Modified(path))
                    if path == "after-cancellation.ini"
            ));
        }
    }

    #[tokio::test]
    async fn wait_preserves_events_and_work_across_repeated_ready_races_and_closure() {
        let root = tempfile::tempdir().expect("root");
        let watcher = WatcherState::new();
        let session = watcher.begin_session(root.path());
        let (sender, mut receiver) = WatchEventReceiver::test_channel();
        for index in 0..32 {
            let path = format!("race-{index}.ini");
            session.request_work();
            sender
                .send(ModWatchEvent::Modified(path.clone()))
                .await
                .expect("event");
            let mut events = 0;
            let mut wakes = 0;
            for _ in 0..2 {
                match tokio::time::timeout(
                    Duration::from_secs(1),
                    wait_for_watcher_activity(&mut receiver, &session, None),
                )
                .await
                .expect("both ready sources must survive")
                {
                    WatcherWaitOutcome::Event(ModWatchEvent::Modified(observed)) => {
                        assert_eq!(observed, path);
                        events += 1;
                    }
                    WatcherWaitOutcome::Work => wakes += 1,
                    _ => panic!("unexpected wait outcome"),
                }
            }
            assert_eq!((events, wakes), (1, 1));
        }
        drop(sender);
        assert!(matches!(
            wait_for_watcher_activity(&mut receiver, &session, None).await,
            WatcherWaitOutcome::Closed
        ));
    }

    #[tokio::test]
    async fn clean_work_wakes_preserve_the_original_backoff_deadline() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState;

        let root = tempfile::tempdir().expect("root");
        let watcher = WatcherState::new();
        let session = watcher.begin_session(root.path());
        let state = DiskReconcileState::new();
        state.begin_authority_session_with_coverage("game", root.path(), &session);
        let mut baseline = startup_result("game");
        state.record_result("game", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game",
            root.path(),
            session.generation(),
            0,
            &baseline,
            &[],
        ));
        let (_sender, mut receiver) = WatchEventReceiver::test_channel();
        let retry_at = tokio::time::Instant::now() + Duration::from_millis(100);
        for _ in 0..2 {
            session.request_work();
            assert!(matches!(
                wait_for_watcher_activity(&mut receiver, &session, Some(retry_at)).await,
                WatcherWaitOutcome::Work
            ));
            assert!(pending_activation_catch_up_plan(
                &state,
                watcher.suppressor.as_ref(),
                "game",
                root.path(),
                &session,
            )
            .is_none());
        }
        assert!(matches!(
            wait_for_watcher_activity(&mut receiver, &session, Some(retry_at)).await,
            WatcherWaitOutcome::Deadline
        ));
        assert!(tokio::time::Instant::now() >= retry_at);
    }

    #[tokio::test]
    async fn ready_idle_wait_reconciles_suppressed_demand_after_last_guard_release_without_event() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            try_reconcile_disk_state_for_prewarm, AuthorityCatchUp, DiskReconcileContext,
            DiskReconcileRequest, DiskReconcileState, InitialRecoveryOutcome,
            InitialRecoveryReadiness,
        };
        use crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileReason;
        use crate::modules::settings::api::config::ConfigService;
        use crate::modules::workspace::application::scanner::watcher::SuppressionGuard;
        use crate::platform::fs::operation_lock::OperationLock;
        use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir_all(root.join("Alice")).expect("object root");
        let root = std::fs::canonicalize(root).expect("canonical root");
        let db = init_test_db().await;
        insert_test_game(
            &db.pool,
            &TestGameFixture {
                id: "game",
                name: "Game",
                game_type: crate::modules::games::api::testing::domain::models::GameType::Gimi,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(root.to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game");
        let config = ConfigService::new_for_test_async(db.pool.clone()).await;
        let state = DiskReconcileState::new();
        let watcher = WatcherState::new();
        let session = watcher.begin_session(&root);
        state.begin_authority_session_with_coverage("game", &root, &session);
        let operation_lock = OperationLock::new();
        let context = DiskReconcileContext {
            pool: &db.pool,
            config: &config,
            state: &state,
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: &operation_lock,
            progress_reporter: None,
        };
        let baseline = try_reconcile_disk_state_for_prewarm(
            context.clone(),
            DiskReconcileRequest::manual(
                "game".into(),
                DiskReconcileReason::StartupBoot,
                Vec::new(),
                true,
            )
            .for_watcher_session(session.clone()),
        )
        .await
        .expect("baseline")
        .expect("baseline lease");
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            0,
            &baseline,
            &[],
        ));
        let recovery = state.mark_initial_recovery_pending("game");
        state.finish_initial_recovery(
            "game",
            recovery,
            InitialRecoveryOutcome::Completed(Box::new(baseline.clone())),
        );
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Ready {
                generation: recovery
            },
        );
        let (_sender, mut receiver) = WatchEventReceiver::test_channel();
        let (waiting, ready) = tokio::sync::oneshot::channel();
        let consume = async {
            waiting.send(()).expect("idle receiver");
            loop {
                match wait_for_watcher_activity(&mut receiver, &session, None).await {
                    WatcherWaitOutcome::Work => {
                        let Some(plan) = pending_activation_catch_up_plan(
                            &state,
                            watcher.suppressor.as_ref(),
                            "game",
                            &root,
                            &session,
                        ) else {
                            continue;
                        };
                        let result = try_reconcile_disk_state_for_prewarm(
                            context.clone(),
                            DiskReconcileRequest::manual(
                                "game".into(),
                                DiskReconcileReason::ManualRepair,
                                plan.changed_paths.clone(),
                                plan.force_full,
                            )
                            .defer_overlay_sync()
                            .for_watcher_session(session.clone()),
                        )
                        .await
                        .expect("wake reconcile")
                        .expect("nonblocking lease");
                        assert!(state.mark_authority_reconciled(
                            "game",
                            &root,
                            session.generation(),
                            plan.observed_generation,
                            &result,
                            &plan.changed_paths,
                        ));
                        break result;
                    }
                    _ => panic!("the receiver has no event or deadline"),
                }
            }
        };
        let mutate = async {
            ready.await.expect("receiver is idle");
            let first = SuppressionGuard::new(&watcher.suppressor);
            let last = SuppressionGuard::new(&watcher.suppressor);
            let mod_path = root.join("Alice/NewSkin");
            std::fs::create_dir(&mod_path).expect("new mod");
            let ini = mod_path.join("mod.ini");
            std::fs::write(&ini, "[TextureOverrideSkin]\nhash = deadbeef\n").expect("new INI");
            state.observe_authority_event("game", session.generation(), &root, &[ini], false);
            watcher.suppressor.mark_blanket_event_dropped(&session);
            drop(first);
            assert!(watcher
                .suppressor
                .load(std::sync::atomic::Ordering::Acquire));
            assert!(watcher.suppressor.pending_repair(&session).is_some());
            drop(last);
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(3), async {
            tokio::join!(consume, mutate)
        })
        .await
        .expect("last guard release must project without another event or refocus");
        assert!(result.status.applied());
        assert!(result.reconcile_revision > baseline.reconcile_revision);
        assert!(watcher.suppressor.pending_repair(&session).is_none());
        let count = db.mod_count("game").await.expect("projected mods");
        assert_eq!(count, 1);
        assert!(matches!(
            state.authority_catch_up("game", &root, session.generation()),
            AuthorityCatchUp::Clean { reconcile_revision, .. }
                if reconcile_revision == result.reconcile_revision
        ));
        assert_eq!(
            state
                .authoritative_result("game")
                .expect("authoritative result")
                .reconcile_revision,
            result.reconcile_revision,
        );
        assert!(operation_lock.try_acquire_for_reconcile().is_some());
        assert!(matches!(
            receiver.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty),
        ));
    }

    #[tokio::test]
    async fn canceled_receiver_lifetime_finishes_pending_recovery() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryReadiness,
        };
        for backoff in [false, true] {
            let root = tempfile::tempdir().expect("owned root");
            let state = Arc::new(DiskReconcileState::new());
            let watcher = WatcherState::new();
            let session = watcher.begin_session(root.path());
            state.begin_authority_session_with_coverage("receiver", root.path(), &session);
            let generation = state.mark_initial_recovery_pending("receiver");
            let (started, ready) = tokio::sync::oneshot::channel();
            let receiver_state = Arc::clone(&state);
            let receiver_root = root.path().to_path_buf();
            let receiver_session = session.clone();
            let (_sender, mut events) = WatchEventReceiver::test_channel();
            let retry_at = backoff.then(|| tokio::time::Instant::now() + Duration::from_secs(30));
            let receiver = tokio::spawn(async move {
                let _lifetime = WatcherReceiverLifetime {
                    state: receiver_state.as_ref(),
                    game_id: "receiver",
                    root: &receiver_root,
                    session: receiver_session.generation(),
                    activation: Some(WatcherActivation {
                        activation_generation: generation,
                        recovery_generation: generation,
                    }),
                    on_end: None,
                };
                started.send(()).expect("lifetime installed");
                wait_for_watcher_activity(&mut events, &receiver_session, retry_at).await;
            });
            ready.await.expect("receiver started");
            receiver.abort();
            assert!(receiver
                .await
                .expect_err("receiver canceled")
                .is_cancelled());
            assert_eq!(
                state.initial_recovery_readiness("receiver"),
                InitialRecoveryReadiness::Failed { generation }
            );
            assert_eq!(
                state.authority_event_generation("receiver", session.generation()),
                Some(1)
            );
            let mut late_result = startup_result("receiver");
            state.record_result("receiver", &mut late_result);
            assert!(!state.mark_authority_reconciled(
                "receiver",
                root.path(),
                session.generation(),
                1,
                &late_result,
                &[]
            ));
            assert!(state.ensure_core_ready_for_mutation("receiver").is_err());
        }
    }

    #[test]
    fn retired_receiver_cannot_finish_a_successor_recovery() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryReadiness,
        };
        let root = tempfile::tempdir().expect("owned root");
        let state = DiskReconcileState::new();
        let watcher = WatcherState::new();
        let old = watcher.begin_session(root.path());
        state.begin_authority_session_with_coverage("receiver", root.path(), &old);
        let generation = state.mark_initial_recovery_pending("receiver");
        let lifetime = WatcherReceiverLifetime {
            state: &state,
            game_id: "receiver",
            root: root.path(),
            session: old.generation(),
            activation: Some(WatcherActivation {
                activation_generation: generation,
                recovery_generation: generation,
            }),
            on_end: None,
        };
        state.reset_initial_recovery("receiver");
        let successor_generation = state.mark_initial_recovery_pending("receiver");
        let successor = watcher.begin_session(root.path());
        state.begin_authority_session_with_coverage("receiver", root.path(), &successor);
        drop(lifetime);
        assert_eq!(
            state.initial_recovery_readiness("receiver"),
            InitialRecoveryReadiness::Syncing {
                generation: successor_generation
            }
        );
        assert_eq!(
            state.authority_event_generation("receiver", successor.generation()),
            Some(0)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn aborting_receiver_owner_releases_the_resolved_onboarding_claim() {
        use crate::modules::reconciliation::api::disk_reconcile::{
            onboarding_session::{ConsumedOnboardingSnapshot, OnboardingIndexingSessionStore},
            orchestrator::{DiskReconcileState, InitialRecoveryReadiness},
        };
        use crate::modules::settings::api::config::GameConfig;

        let temp = tempfile::tempdir().expect("root");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods root");
        let store = OnboardingIndexingSessionStore::new();
        let onboarding = store
            .begin(vec![GameConfig {
                id: "game".into(),
                name: "Game".into(),
                game_type: crate::modules::games::api::testing::domain::models::GameType::Gimi,
                instance_path: temp.path().to_path_buf(),
                mod_path: root.clone(),
                ready_to_move_path: None,
                launch_mode: Default::default(),
                game_exe: None,
                loader_exe: None,
                xxmi_launcher_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }])
            .await
            .expect("onboarding snapshot");
        let (claimed, claim_guard) = store
            .claim_for_activation("game")
            .expect("activation claim");
        let ConsumedOnboardingSnapshot::Snapshot(snapshot_lease) =
            claimed.resolve().await.expect("resolved raw snapshot")
        else {
            panic!("unchanged snapshot must retain its raw lease");
        };
        assert!(snapshot_lease.matches_root(&root));
        assert!(store.is_claimed_by_activation(&onboarding.session_id, "game"));
        let state = Arc::new(DiskReconcileState::new());
        let watcher = WatcherState::new();
        let session = watcher.begin_session(&root);
        state.begin_authority_session_with_coverage("game", &root, &session);
        let recovery = state.mark_initial_recovery_pending("game");
        let receiver_state = Arc::clone(&state);
        let generation = session.generation();
        let (started, ready) = tokio::sync::oneshot::channel();
        let owner = tokio::spawn(async move {
            let _lifetime = WatcherReceiverLifetime {
                state: receiver_state.as_ref(),
                game_id: "game",
                root: &root,
                session: generation,
                activation: Some(WatcherActivation {
                    activation_generation: recovery,
                    recovery_generation: recovery,
                }),
                on_end: None,
            };
            let _claim_guard = claim_guard;
            let _snapshot_lease = snapshot_lease;
            started.send(()).expect("owner started");
            std::future::pending::<()>().await;
        });
        ready
            .await
            .expect("raw snapshot and claim are held by the receiver owner");
        let wait_for_release = store.wait_for_activation_claim(&onboarding.session_id, "game");
        tokio::pin!(wait_for_release);
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut wait_for_release)
                .await
                .is_err()
        );
        owner.abort();
        assert!(owner.await.expect_err("owner canceled").is_cancelled());
        tokio::time::timeout(Duration::from_secs(1), &mut wait_for_release)
            .await
            .expect("background claim waiter resumes after task cancellation");
        assert!(!store.is_claimed_by_activation(&onboarding.session_id, "game"));
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Failed {
                generation: recovery
            },
        );
        assert_eq!(
            state.authority_event_generation("game", generation),
            Some(1)
        );
        let mut late_result = startup_result("game");
        state.record_result("game", &mut late_result);
        assert!(!state.mark_authority_reconciled(
            "game",
            &temp.path().join("Mods"),
            generation,
            1,
            &late_result,
            &[]
        ));
        assert!(state.ensure_core_ready_for_mutation("game").is_err());
        store.cancel(&onboarding.session_id).expect("cleanup");
    }

    #[tokio::test]
    async fn canceled_reconcile_releases_owned_leases_and_allows_the_next_ordinary_batch() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            reconcile_disk_state_for_watcher, try_reconcile_disk_state_for_prewarm,
            DiskReconcileContext, DiskReconcileRequest, DiskReconcileState,
        };
        use crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileReason;
        use crate::modules::settings::api::config::ConfigService;
        use crate::platform::fs::operation_lock::OperationLock;
        use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

        let temp = tempfile::tempdir().expect("root");
        let root = temp.path().join("Mods");
        let mod_path = root.join("Alice/NewSkin");
        std::fs::create_dir_all(&mod_path).expect("mod root");
        std::fs::write(
            mod_path.join("mod.ini"),
            "[TextureOverrideSkin]\nhash = deadbeef\n",
        )
        .expect("mod INI");
        let root = std::fs::canonicalize(root).expect("canonical root");
        let db = init_test_db().await;
        insert_test_game(
            &db.pool,
            &TestGameFixture {
                id: "game",
                name: "Game",
                game_type: crate::modules::games::api::testing::domain::models::GameType::Gimi,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(root.to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game");
        let config = ConfigService::new_for_test_async(db.pool.clone()).await;
        for waiting_for_operation in [false, true] {
            let state = DiskReconcileState::new();
            let watcher = WatcherState::new();
            let session = watcher.begin_session(&root);
            state.begin_authority_session_with_coverage("game", &root, &session);
            state.observe_authority_event(
                "game",
                session.generation(),
                &root,
                &[root.join("Alice")],
                false,
            );
            watcher.suppressor.mark_blanket_event_dropped(&session);
            let observed = state.authority_event_generation("game", session.generation());
            let operation_lock = OperationLock::new();
            let held_game = if waiting_for_operation {
                None
            } else {
                Some(state.try_game_guard("game").expect("foreground game lease"))
            };
            let held_operation = if waiting_for_operation {
                Some(
                    operation_lock
                        .acquire()
                        .await
                        .expect("foreground operation"),
                )
            } else {
                None
            };
            let context = DiskReconcileContext {
                pool: &db.pool,
                config: &config,
                state: &state,
                watcher_suppressor: watcher.suppressor.clone(),
                operation_lock: &operation_lock,
                progress_reporter: None,
            };
            let mut reconcile = Box::pin(reconcile_disk_state_for_watcher(
                context.clone(),
                DiskReconcileRequest::manual(
                    "game".into(),
                    DiskReconcileReason::WatcherBatch,
                    Vec::new(),
                    true,
                ),
                &watcher,
                session.clone(),
            ));
            assert!(
                tokio::time::timeout(Duration::from_millis(10), &mut reconcile)
                    .await
                    .is_err()
            );
            assert!(state.try_game_guard("game").is_none());
            drop(reconcile);
            assert_eq!(
                state.try_game_guard("game").is_some(),
                waiting_for_operation
            );
            assert_eq!(
                operation_lock.try_acquire_for_reconcile().is_some(),
                !waiting_for_operation
            );
            assert!(state.authoritative_result("game").is_none());
            assert_eq!(
                state.authority_event_generation("game", session.generation()),
                observed
            );
            assert!(watcher.suppressor.pending_repair(&session).is_some());
            drop(held_game);
            drop(held_operation);

            let batch = vec![ModWatchEvent::Created(
                root.join("Alice/NewSkin").to_string_lossy().into_owned(),
            )];
            let paths = crate::modules::reconciliation::api::disk_reconcile::watcher_batch::collect_changed_paths(&batch);
            let result = try_reconcile_disk_state_for_prewarm(
                context,
                DiskReconcileRequest::watcher_batch("game".into(), &root, paths.clone(), &batch)
                    .for_watcher_session(session.clone()),
            )
            .await
            .expect("ordinary projection after cancellation")
            .expect("ordinary nonblocking lease");
            assert!(result.status.applied());
            assert!(
                state.try_game_guard("game").is_some()
                    && operation_lock.try_acquire_for_reconcile().is_some(),
                "ordinary projection returns with both leases released before runtime work can enqueue",
            );
            assert!(state.mark_authority_reconciled(
                "game",
                &root,
                session.generation(),
                observed.expect("observed generation"),
                &result,
                &paths,
            ));
            let count = db
                .mod_count("game")
                .await
                .expect("ordinary projection rows");
            assert_eq!(count, 1);
        }
    }

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

    fn startup_result(
        game_id: &str,
    ) -> crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileResult {
        use crate::modules::reconciliation::api::disk_reconcile::types::{
            DiskReconcileReason, DiskReconcileScanScope, DiskReconcileStatus,
        };
        crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileResult {
            game_id: game_id.to_string(),
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
        }
    }

    #[tokio::test]
    async fn current_reuse_rejects_replaced_runtime_parent_with_unbound_ready_gate() {
        use crate::modules::mutation::api::MutationCoordinator;
        use crate::modules::mutation::journal::OperationJournal;
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryOutcome,
        };
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path().join("Mods");
        let importer = temp.path().join("Importer");
        std::fs::create_dir(&root).expect("mods");
        std::fs::create_dir(&importer).expect("importer");
        let runtime = importer.join("d3dx.ini");
        let watcher = WatcherState::new();
        let (session, _receiver, _) = replace_watcher(&watcher, &root, Some(&runtime), |session| {
            crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config(
                &root,
                Some(&runtime),
                watcher.suppressor.clone(),
                session,
            )
        })
        .expect("installed coverage");
        let state = DiskReconcileState::new();
        state.begin_authority_session_with_coverage("game", &root, &session);
        let mut baseline = startup_result("game");
        state.record_result("game", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            0,
            &baseline,
            &[]
        ));
        let recovery = state.mark_initial_recovery_pending("game");
        assert!(state.finish_initial_recovery(
            "game",
            recovery,
            InitialRecoveryOutcome::Completed(Box::new(baseline))
        ));
        let generation = state.begin_activation(Some("game".into()));
        let coordinator = MutationCoordinator::with_lock(
            crate::platform::fs::operation_lock::OperationLock::new(),
            Arc::new(
                OperationJournal::open(temp.path().join("journal.json"), 100).expect("journal"),
            ),
        );
        assert!(prepare_current_activation_reuse(
            &state,
            &watcher,
            &coordinator,
            "game",
            &root,
            &runtime,
            generation
        )
        .is_some());
        std::fs::rename(&importer, temp.path().join("RetiredImporter"))
            .expect("replace runtime parent");
        std::fs::create_dir(&importer).expect("new importer");
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                generation
            )
            .is_none(),
            "same lexical runtime path cannot reuse coverage of the previous physical parent"
        );
        assert!(watcher.is_current_session(&session));
    }

    #[tokio::test]
    async fn startup_same_game_reuse_keeps_installed_session_and_acceptance_lease() {
        use crate::modules::mutation::api::MutationCoordinator;
        use crate::modules::mutation::journal::OperationJournal;
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryOutcome, InitialRecoveryReadiness,
        };
        use crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileScanScope;

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods");
        let runtime = temp.path().join("d3dx.ini");
        let watcher = WatcherState::new();
        let (session, mut receiver, _) =
            replace_watcher(&watcher, &root, Some(&runtime), |session| {
                crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config(
                    &root,
                    Some(&runtime),
                    watcher.suppressor.clone(),
                    session,
                )
            })
            .expect("startup watcher");
        let state = DiskReconcileState::new();
        state.begin_authority_session_with_coverage("game", &root, &session);
        let mut baseline = startup_result("game");
        state.record_result("game", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            0,
            &baseline,
            &[]
        ));
        let recovery = state.mark_initial_recovery_pending_for_root("game", &root, &runtime);
        state.finish_initial_recovery(
            "game",
            recovery,
            InitialRecoveryOutcome::Completed(Box::new(baseline.clone())),
        );
        let generation = state.begin_activation(Some("game".to_string()));
        let coordinator = MutationCoordinator::with_lock(
            crate::platform::fs::operation_lock::OperationLock::new(),
            Arc::new(
                OperationJournal::open(temp.path().join("journal.json"), 100).expect("journal"),
            ),
        );
        let reused = prepare_current_activation_reuse(
            &state,
            &watcher,
            &coordinator,
            "game",
            &root,
            &runtime,
            generation,
        )
        .expect("reuse installed startup coverage");
        assert_eq!(reused.session(), &session);
        assert_eq!(reused.activation().recovery_generation, recovery);
        assert_eq!(
            reused.result().reconcile_revision,
            baseline.reconcile_revision
        );
        assert_eq!(reused.result().scan_scope, DiskReconcileScanScope::None);
        assert_eq!(
            state.initial_recovery_readiness("game"),
            InitialRecoveryReadiness::Ready {
                generation: recovery
            }
        );
        assert!(
            coordinator
                .inner_lock()
                .try_acquire_for_reconcile()
                .is_none(),
            "publication owns the acceptance lease"
        );
        assert!(watcher.is_current_session(&session));
        drop(reused);
        assert!(
            watcher
                .inactive_watcher_session("game", &root, Some(&runtime))
                .is_none(),
            "the previous inactive-only gate would reject this installed startup watcher"
        );
        let foreground = coordinator
            .inner_lock()
            .acquire()
            .await
            .expect("foreground lease");
        assert!(prepare_current_activation_reuse(
            &state,
            &watcher,
            &coordinator,
            "game",
            &root,
            &runtime,
            generation
        )
        .is_none());
        assert_eq!(
            state
                .joinable_initial_recovery_binding("game", &root, &runtime)
                .map(|(generation, _)| generation),
            Some(recovery),
            "a completed physical owner remains joinable while foreground work holds acceptance"
        );
        drop(foreground);
        assert_created_event(&mut receiver, &root.join("during-reuse.ini")).await;

        assert!(prepare_current_activation_reuse(
            &state,
            &watcher,
            &coordinator,
            "game",
            &root,
            &temp.path().join("other.ini"),
            generation
        )
        .is_none());
        let newer = state.begin_activation(Some("game".to_string()));
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                generation
            )
            .is_none(),
            "stale generation cannot reuse"
        );
        state.observe_authority_event(
            "game",
            session.generation(),
            &root,
            &[root.join("Changed")],
            false,
        );
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                newer
            )
            .is_none(),
            "dirty evidence prevents reuse"
        );
        assert!(watcher.is_current_session(&session));
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            state
                .authority_event_generation("game", session.generation())
                .expect("observed"),
            &baseline,
            &[root.join("Changed").to_string_lossy().into_owned()]
        ));
        watcher.suppressor.mark_blanket_event_dropped(&session);
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                newer
            )
            .is_none(),
            "suppressor repair cannot be hidden by current-session reuse"
        );
    }

    #[tokio::test]
    async fn current_activation_reuse_rejects_replaced_root_and_missing_watcher() {
        use crate::modules::mutation::api::MutationCoordinator;
        use crate::modules::mutation::journal::OperationJournal;
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryOutcome,
        };
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods");
        let runtime = temp.path().join("d3dx.ini");
        let watcher = WatcherState::new();
        let (session, _receiver, _) = replace_watcher(&watcher, &root, Some(&runtime), |session| {
            crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config(
                &root,
                Some(&runtime),
                watcher.suppressor.clone(),
                session,
            )
        })
        .expect("watcher");
        let state = DiskReconcileState::new();
        state.begin_authority_session_with_coverage("game", &root, &session);
        let mut baseline = startup_result("game");
        state.record_result("game", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            0,
            &baseline,
            &[]
        ));
        let recovery = state.mark_initial_recovery_pending("game");
        let generation = state.begin_activation(Some("game".into()));
        let coordinator = MutationCoordinator::with_lock(
            crate::platform::fs::operation_lock::OperationLock::new(),
            Arc::new(
                OperationJournal::open(temp.path().join("journal.json"), 100).expect("journal"),
            ),
        );
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                generation
            )
            .is_none(),
            "a clean snapshot without completed recovery cannot reuse"
        );
        state.finish_initial_recovery(
            "game",
            recovery,
            InitialRecoveryOutcome::Completed(Box::new(baseline)),
        );
        let installed = lock(&watcher.watcher).take().expect("installed");
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                generation
            )
            .is_none(),
            "a published session without a native watcher cannot reuse"
        );
        *lock(&watcher.watcher) = Some(installed);
        std::fs::rename(&root, temp.path().join("RetiredMods")).expect("move original root");
        std::fs::create_dir(&root).expect("replacement root");
        assert!(
            prepare_current_activation_reuse(
                &state,
                &watcher,
                &coordinator,
                "game",
                &root,
                &runtime,
                generation
            )
            .is_none(),
            "same path cannot rebind a replacement physical root"
        );
        assert!(watcher.is_current_session(&session));
    }

    #[tokio::test]
    async fn activation_ready_rechecks_actual_cache_after_reuse_capture() {
        use crate::modules::mutation::api::MutationCoordinator;
        use crate::modules::mutation::journal::OperationJournal;
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            DiskReconcileState, InitialRecoveryOutcome,
        };

        let temp = tempfile::tempdir().expect("root");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("mods");
        let runtime = temp.path().join("d3dx.ini");
        let watcher = WatcherState::new();
        let (session, _receiver, _) = replace_watcher(&watcher, &root, Some(&runtime), |session| {
            crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config(
                &root, Some(&runtime), watcher.suppressor.clone(), session,
            )
        })
        .expect("watcher");
        let state = DiskReconcileState::new();
        state.begin_authority_session_with_coverage("game", &root, &session);
        let mut baseline = startup_result("game");
        state.record_result("game", &mut baseline);
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            0,
            &baseline,
            &[],
        ));
        let recovery = state.mark_initial_recovery_pending("game");
        state.finish_initial_recovery(
            "game",
            recovery,
            InitialRecoveryOutcome::Completed(Box::new(baseline)),
        );
        let activation = state.begin_activation(Some("game".into()));
        let coordinator = MutationCoordinator::with_lock(
            crate::platform::fs::operation_lock::OperationLock::new(),
            Arc::new(
                OperationJournal::open(temp.path().join("journal.json"), 100).expect("journal"),
            ),
        );
        let captured = prepare_current_activation_reuse(
            &state,
            &watcher,
            &coordinator,
            "game",
            &root,
            &runtime,
            activation,
        )
        .expect("captured reuse");
        let publication_allowed = || {
            activation_publication_allowed(captured.result(), || {
                activation_result_is_accepted(
                    &state,
                    watcher.suppressor.as_ref(),
                    &coordinator,
                    "game",
                    &root,
                    captured.session(),
                    captured.result(),
                )
            })
        };
        assert!(publication_allowed());
        session
            .identity_cache_validity()
            .store(false, std::sync::atomic::Ordering::Release);
        assert!(watcher.is_current_session(captured.session()));
        assert!(coordinator
            .inner_lock()
            .try_acquire_for_reconcile()
            .is_none());
        assert!(
            !publication_allowed(),
            "Ready must reject invalidation of the actual captured resource under its held lease",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn superseded_activation_retry_preserves_dirty_plan_and_projects_receiver_events() {
        use crate::modules::reconciliation::api::disk_reconcile::orchestrator::{
            try_reconcile_disk_state_for_prewarm, AuthorityCatchUp, DiskReconcileContext,
            DiskReconcileRequest, DiskReconcileState,
        };
        use crate::modules::reconciliation::api::disk_reconcile::types::DiskReconcileReason;
        use crate::modules::settings::api::config::ConfigService;
        use crate::platform::fs::operation_lock::OperationLock;
        use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        std::fs::create_dir_all(root.join("Alice")).expect("object root");
        let root = std::fs::canonicalize(&root).expect("canonical root");
        let db = init_test_db().await;
        insert_test_game(
            &db.pool,
            &TestGameFixture {
                id: "game",
                name: "Game",
                game_type: crate::modules::games::api::testing::domain::models::GameType::Gimi,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(root.to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game");
        let config = ConfigService::new_for_test_async(db.pool.clone()).await;
        let state = Arc::new(DiskReconcileState::new());
        let watcher = WatcherState::new();
        let (session, mut receiver, _) = replace_watcher(&watcher, &root, None, |session| {
            let observed_state = state.clone();
            let observed_root = root.clone();
            let generation = session.generation();
            crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config_and_observer(
                &root,
                None,
                watcher.suppressor.clone(),
                session,
                Some(Arc::new(move |paths, _, lost| {
                    observed_state.observe_authority_event(
                        "game",
                        generation,
                        &observed_root,
                        paths,
                        lost,
                    );
                })),
            )
        })
        .expect("watcher");
        state.begin_authority_session_with_coverage("game", &root, &session);
        let operation_lock = OperationLock::new();
        let context = DiskReconcileContext {
            pool: &db.pool,
            config: &config,
            state: state.as_ref(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: &operation_lock,
            progress_reporter: None,
        };
        let baseline = try_reconcile_disk_state_for_prewarm(
            context.clone(),
            DiskReconcileRequest::manual(
                "game".into(),
                DiskReconcileReason::StartupBoot,
                Vec::new(),
                true,
            )
            .for_watcher_session(session.clone()),
        )
        .await
        .expect("baseline")
        .expect("baseline lease");
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            0,
            &baseline,
            &[]
        ));
        let recovery = state.mark_initial_recovery_pending("game");
        let generation = state.begin_activation(Some("game".into()));
        let mut duty = Some(WatcherActivation {
            activation_generation: generation,
            recovery_generation: recovery,
        });
        let mut activation_recovery_pending = true;
        let mut pending_catch_up = None;
        let mut retry_after = Some(tokio::time::Instant::now() + Duration::from_secs(30));
        let authority = WatcherAuthorityContext {
            state: &state,
            suppressor: watcher.suppressor.as_ref(),
            game_id: "game",
            mods_root: &root,
            session: &session,
        };
        assert!(!authority.retire_publication(
            &mut duty,
            &mut activation_recovery_pending,
            &mut pending_catch_up,
            &mut retry_after
        ));
        assert!(retry_after.is_some());
        state.observe_authority_event(
            "game",
            session.generation(),
            &root,
            &[root.join("Alice")],
            false,
        );
        pending_catch_up = pending_activation_catch_up_plan(
            &state,
            watcher.suppressor.as_ref(),
            "game",
            &root,
            &session,
        );
        state.begin_activation(Some("game".into()));
        assert!(authority.retire_publication(
            &mut duty,
            &mut activation_recovery_pending,
            &mut pending_catch_up,
            &mut retry_after
        ));
        assert!(duty.is_none());
        assert!(!activation_recovery_pending);
        assert!(
            retry_after.is_none(),
            "the obsolete publication-only timer must be cleared"
        );
        assert!(watcher.is_current_session(&session));
        let retained = pending_catch_up
            .take()
            .expect("dirty plan survives retirement");
        assert!(!retained.force_full);
        assert_eq!(
            retained.changed_paths,
            vec![root.join("Alice").to_string_lossy().into_owned()]
        );
        let repaired = try_reconcile_disk_state_for_prewarm(
            context.clone(),
            DiskReconcileRequest::manual(
                "game".into(),
                DiskReconcileReason::ManualRepair,
                retained.changed_paths.clone(),
                retained.force_full,
            )
            .for_watcher_session(session.clone()),
        )
        .await
        .expect("retained retry")
        .expect("retry lease");
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            retained.observed_generation,
            &repaired,
            &retained.changed_paths
        ));

        let new_mod = root.join("Alice/NewSkin");
        std::fs::create_dir(&new_mod).expect("new mod");
        let ini = new_mod.join("mod.ini");
        std::fs::write(&ini, "[TextureOverrideSkin]\nhash = deadbeef\n").expect("new ini");
        let batch = tokio::time::timeout(Duration::from_secs(3), async {
            let mut events = Vec::new();
            loop {
                let event = receiver.recv().await.expect("retained receiver");
                let observed_ini = matches!(&event, ModWatchEvent::Created(path) | ModWatchEvent::Modified(path) if Path::new(path) == ini);
                events.push(event);
                if observed_ini { break; }
            }
            while let Ok(event) = receiver.try_recv() { events.push(event); }
            events
        }).await.expect("new event must reach retained receiver");
        let paths = crate::modules::reconciliation::api::disk_reconcile::watcher_batch::collect_changed_paths(&batch);
        let observed = state
            .authority_event_generation("game", session.generation())
            .expect("observed generation");
        let projected = try_reconcile_disk_state_for_prewarm(
            context,
            DiskReconcileRequest::watcher_batch("game".into(), &root, paths.clone(), &batch)
                .for_watcher_session(session.clone()),
        )
        .await
        .expect("receiver event reconcile")
        .expect("event lease");
        assert!(projected.reconcile_revision > repaired.reconcile_revision);
        assert!(state.mark_authority_reconciled(
            "game",
            &root,
            session.generation(),
            observed,
            &projected,
            &paths
        ));
        let mod_count = db.mod_count("game").await.expect("projected mods");
        assert_eq!(
            mod_count, 1,
            "native observation reaches disk projection after old activation retirement"
        );
        assert!(
            matches!(state.authority_catch_up("game", &root, session.generation()), AuthorityCatchUp::Clean { reconcile_revision, .. } if reconcile_revision == projected.reconcile_revision)
        );
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
            &session,
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
        assert!(activation_result_is_accepted(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
            &baseline,
        ));
        assert!(initial_activation_waits_for_barrier(true, true, &baseline));
        let mut unavailable = baseline.clone();
        unavailable.status = DiskReconcileStatus::SourceUnavailable;
        assert!(
            !initial_activation_waits_for_barrier(true, true, &unavailable),
            "a genuinely unavailable source must still publish its diagnostic status"
        );
        assert!(try_clean_activation_recovery_result(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
        )
        .is_some());

        let guard = coordinator.inner_lock().acquire().await.expect("lock");
        assert!(
            try_clean_activation_recovery_result(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
            )
            .is_none(),
            "background readiness polling must yield to a foreground operation"
        );
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
        assert!(
            try_clean_activation_recovery_result(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
            )
            .is_none(),
            "retry must not report Ready while disk commit is pending"
        );
        assert!(!activation_result_is_accepted(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
            &baseline,
        ));
        assert!(
            pending_watcher_catch_up_plan(&state, "game-1", &mods_root, session.generation(),)
                .is_none(),
            "pending commit must recheck the barrier without another disk scan"
        );
        journal.mark_db_committed(&id).expect("database projection");
        journal.complete(&id).expect("settled operation");
        assert!(
            try_clean_activation_recovery_result(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
            )
            .is_some(),
            "a clean settled result can recover without a second scan"
        );
        let (stale_candidate, _guard) = try_clean_activation_recovery_result(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
        )
        .expect("candidate before a new watcher event");
        state.observe_authority_event(
            "game-1",
            session.generation(),
            &mods_root,
            &[mods_root.join("External")],
            false,
        );
        let recovery_generation = state.mark_initial_recovery_pending("game-1");
        assert!(!activation_publication_allowed(&stale_candidate, || {
            activation_result_is_accepted(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
                &stale_candidate,
            )
        }));
        assert!(
            matches!(
                state.initial_recovery_readiness("game-1"),
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Syncing { generation }
                    if generation == recovery_generation
            ),
            "rejected publication must not finish the recovery gate"
        );
        assert!(
            !activation_result_is_accepted(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
                &stale_candidate,
            ),
            "a watcher event between candidate read and publication must prevent Ready"
        );
        assert!(pending_activation_catch_up_plan(
            &state,
            watcher.suppressor.as_ref(),
            "game-1",
            &mods_root,
            &session,
        )
        .is_some());
        drop(_guard);

        watcher.suppressor.mark_blanket_event_dropped(&session);
        assert!(cached().is_none());
        let repair_plan = pending_activation_catch_up_plan(
            &state,
            watcher.suppressor.as_ref(),
            "game-1",
            &mods_root,
            &session,
        )
        .expect("dropped watcher events require a full repair scan");
        assert!(repair_plan.force_full);
        assert!(
            try_clean_activation_recovery_result(
                &state,
                watcher.suppressor.as_ref(),
                &coordinator,
                "game-1",
                &mods_root,
                &session,
            )
            .is_none(),
            "retry must not report Ready while suppressor repair is pending"
        );
        assert!(!activation_result_is_accepted(
            &state,
            watcher.suppressor.as_ref(),
            &coordinator,
            "game-1",
            &mods_root,
            &session,
            &baseline,
        ));
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
