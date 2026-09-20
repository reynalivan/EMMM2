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
) -> Result<u64, ScannerError> {
    if let Some(session) = state.inactive_watcher_session(game_id, mods_root, runtime_config_path) {
        return Ok(session);
    }
    if state.discard_inactive_watcher_unless_coverage(game_id, mods_root, runtime_config_path) {
        app.state::<
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
        >()
        .invalidate_authority(game_id, mods_root);
    }
    let active_watcher = lock(&state.watcher);
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
    let (watcher, mut receiver) = match watcher_result {
        Ok(installed) => installed,
        Err(error) => {
            app.state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >()
            .invalidate_authority(game_id, mods_root);
            return Err(error);
        }
    };
    let reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    state.install_inactive_watcher(
        game_id.to_string(),
        mods_root,
        runtime_config_path,
        session.generation(),
        watcher,
    );
    let continuity_proven = covered_session.is_some_and(|covered_session| {
        reconcile_state.handoff_authority_session(
            game_id,
            mods_root,
            covered_session,
            session.generation(),
        )
    });
    if !continuity_proven {
        reconcile_state.begin_authority_session(game_id, mods_root, session.generation());
    }
    drop(active_watcher);
    let app_for_close = app.clone();
    let game_for_close = game_id.to_string();
    let root_for_close = mods_root.to_path_buf();
    let session_for_close = session.generation();
    tokio::spawn(async move {
        while receiver.recv().await.is_some() {}
        app_for_close
            .state::<WatcherState>()
            .remove_inactive_watcher_if_session(&game_for_close, session_for_close);
        app_for_close
            .state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >()
            .observe_authority_event(
                &game_for_close,
                session_for_close,
                &root_for_close,
                &[],
                true,
            );
    });
    Ok(session.generation())
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
                Ok(session) => session,
                Err(error) => {
                    log::warn!("Could not prewarm watcher for '{}': {error}", game.id);
                    continue;
                }
            };
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
            let result = tokio::time::timeout(
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
            )
            .await;
            match result {
                Ok(Ok(Some(result))) => {
                    reconcile_state.mark_authority_reconciled(
                        &game.id,
                        &game.mod_path,
                        watcher_session,
                        observed_generation,
                        &result,
                        &changed_paths,
                    );
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
    start_watcher_inner(app, state, pool, path, game_id, None)
}

pub fn start_watcher_for_activation(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
    activation: WatcherActivation,
) -> Result<(), ScannerError> {
    start_watcher_inner(app, state, pool, path, game_id, Some(activation))
}

fn start_watcher_inner(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
    activation: Option<WatcherActivation>,
) -> Result<(), ScannerError> {
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

    let replacement = replace_watcher(state, path_obj, runtime_config_path.as_deref(), |session| {
        let observer =
            authority_observer(&app, &game_id, path_obj, &session, state.suppressor.clone());
        crate::modules::workspace::application::scanner::watcher::watch_mod_directory_with_runtime_config_and_observer(
            path_obj,
            runtime_config_path.as_deref(),
            state.suppressor.clone(),
            session,
            Some(observer),
        )
    });
    let disk_reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let (session, rx, covered_active_session) = match replacement {
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
    let inactive_handoff =
        state.take_inactive_watcher_for_handoff(&game_id, path_obj, runtime_config_path.as_deref());
    let continuity_proven = inactive_handoff
        .as_ref()
        .is_some_and(|(covered_session, _)| {
            disk_reconcile_state.handoff_authority_session(
                &game_id,
                path_obj,
                *covered_session,
                session.generation(),
            )
        })
        || covered_active_session.is_some_and(|covered_session| {
            disk_reconcile_state.handoff_authority_session(
                &game_id,
                path_obj,
                covered_session,
                session.generation(),
            )
        });
    if !continuity_proven {
        disk_reconcile_state.begin_authority_session(&game_id, path_obj, session.generation());
    }
    drop(inactive_handoff);

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

    Ok(())
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
    // Activation can inherit continuous coverage from an inactive watcher.
    // Otherwise the authority plan below treats this session as an event gap
    // and verifies the whole source before accepting scoped observations.
    let disk_reconcile_state =
        app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
    let config = app.state::<crate::modules::settings::application::config::ConfigService>();
    let operation_lock = app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
    let watcher_state = app.state::<WatcherState>();
    let mods_root = std::path::Path::new(&mods_path_root);
    let watcher_session = session.generation();
    let catch_up = if activation.is_some() {
        disk_reconcile_state.authority_catch_up(&game_id, mods_root, watcher_session)
    } else {
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Full {
            observed_generation: disk_reconcile_state
                .authority_event_generation(&game_id, watcher_session)
                .unwrap_or(0),
        }
    };
    let (cached_result, recovery_changed_paths, recovery_force_full, recovery_generation) =
        match catch_up {
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Clean {
                observed_generation,
                ..
            } if disk_reconcile_state.authority_event_generation(&game_id, watcher_session)
                == Some(observed_generation) => {
                let cached = disk_reconcile_state.authoritative_result(&game_id).map(|mut result| {
                    result.scan_scope = crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::None;
                    result
                });
                (cached, Vec::new(), false, observed_generation)
            }
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Scoped {
                changed_paths,
                observed_generation,
            } => (None, changed_paths, false, observed_generation),
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Full {
                observed_generation,
            }
            | crate::modules::reconciliation::application::disk_reconcile::orchestrator::AuthorityCatchUp::Clean {
                observed_generation,
                ..
            } => (None, Vec::new(), true, observed_generation),
        };
    let session_recovery = if let Some(result) = cached_result {
        Ok(crate::modules::reconciliation::application::disk_reconcile::orchestrator::WatcherReconcileOutcome::Applied(result))
    } else {
        let recovery_paths = recovery_changed_paths.clone();
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
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                    )
                    .for_watcher_session(session.clone()),
                )),
            },
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                game_id.clone(),
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                recovery_paths,
                recovery_force_full,
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
                disk_reconcile_state.mark_authority_reconciled(
                    &game_id,
                    mods_root,
                    watcher_session,
                    observed_generation,
                    &result,
                    &changed_paths,
                );
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
    .observe_authority_event(
        &game_id,
        watcher_session,
        mods_root,
        &[],
        true,
    );
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
