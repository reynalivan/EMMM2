use crate::shared::errors::AppError;
use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileReason, DiskReconcileStatus,
};
use crate::modules::system::application::app::post_apply::RuntimeSyncRequest;

#[derive(Default)]
struct OnboardingSnapshotTelemetryState {
    preparation_started_at: Option<Instant>,
}

fn snapshot_rechecking_progress(
    session_id: String,
    game_id: String,
) -> crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotProgress{
    crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotProgress {
        session_id,
        game_id,
        phase: crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotPhase::Rechecking,
        completed_games: 0,
        total_games: 0,
        completed_roots: 0,
        total_roots: 0,
        folders_classified: 0,
        current_root: None,
        elapsed_ms: 0,
    }
}

fn record_onboarding_telemetry(
    sink: &Option<crate::modules::system::application::telemetry::TelemetrySink>,
    operation: crate::modules::system::application::telemetry::TelemetryOperation,
    outcome: crate::modules::system::application::telemetry::TelemetryOutcome,
    error_code: crate::modules::system::application::telemetry::TelemetryErrorCode,
    duration: Duration,
) {
    if let Some(sink) = sink {
        sink.try_enqueue([
            crate::modules::system::application::telemetry::TelemetryEvent::new(
                operation, outcome, error_code,
            )
            .with_duration(duration),
        ]);
    }
}

fn emit_onboarding_background_status(
    app: &tauri::AppHandle,
    status: crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundStatus,
) {
    if let Err(error) = app.emit("onboarding_indexing:background_status", status) {
        log::debug!("Could not emit onboarding background indexing status: {error}");
    }
}

fn update_onboarding_background_phase(
    app: &tauri::AppHandle,
    sessions: &crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
    session_id: &str,
    game_id: &str,
    phase: crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase,
) -> Result<(), AppError> {
    let status = sessions.set_background_phase(session_id, game_id, phase)?;
    emit_onboarding_background_status(app, status);
    Ok(())
}

fn background_phase_for_reconcile_status(
    status: &DiskReconcileStatus,
) -> crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase{
    if status.applied() {
        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::Ready
    } else {
        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::NeedsAttention
    }
}

fn telemetry_outcome_for_error(
    error: &AppError,
) -> crate::modules::system::application::telemetry::TelemetryOutcome {
    if matches!(error, AppError::Cancelled) {
        crate::modules::system::application::telemetry::TelemetryOutcome::Cancelled
    } else {
        crate::modules::system::application::telemetry::TelemetryOutcome::Failed
    }
}

fn checked_resolution_path(root: &Path, relative: &str) -> Result<String, AppError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::Security(
            "Rename confirmation path must be a contained relative folder path".to_string(),
        ));
    }
    Ok(root.join(path).to_string_lossy().to_string())
}

fn should_wait_for_initial_recovery(
    reason: &DiskReconcileReason,
    readiness: InitialRecoveryReadiness,
) -> bool {
    matches!(
        reason,
        DiskReconcileReason::ModsViewEntered | DiskReconcileReason::GameSwitched
    ) && matches!(readiness, InitialRecoveryReadiness::Syncing { .. })
}

fn manual_reconcile_runtime_request(changed_roots: &[String]) -> RuntimeSyncRequest {
    if changed_roots.is_empty() {
        RuntimeSyncRequest::Full
    } else {
        crate::modules::reconciliation::api::runtime_sync_request_for_roots(changed_roots)
    }
}

fn enqueue_runtime_sync_after_manual_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    result: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
) {
    if !result.status.applied()
        || (!result.folders_changed && !result.runtime_file_changed)
        || config.get_settings().active_game_id.as_deref() != Some(result.game_id.as_str())
    {
        return;
    }

    let request = manual_reconcile_runtime_request(&result.changed_roots);
    match request {
        RuntimeSyncRequest::Full => crate::modules::reconciliation::api::enqueue_runtime_sync(
            app,
            pool,
            &result.game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
        ),
        request => crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
            app,
            pool,
            &result.game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
            request,
        ),
    };
}

#[tauri::command]
#[specta::specta]
pub async fn inspect_game_mods_directory(
    game_id: String,
    candidate_path: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::source_recovery::GameModsDirectoryInspection, AppError>
{
    crate::modules::reconciliation::application::disk_reconcile::source_recovery::inspect_game_mods_directory(
        pool.inner(),
        &game_id,
        Path::new(&candidate_path),
    )
    .await
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn apply_game_mods_directory(
    app: tauri::AppHandle,
    request: crate::modules::reconciliation::application::disk_reconcile::source_recovery::ApplyGameModsDirectoryRequest,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    operation_lock: State<'_, crate::modules::mutation::coordinator::MutationCoordinator>,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::source_recovery::ApplyGameModsDirectoryResult, AppError>
{
    let game_id = request.game_id.clone();
    let _activation_guard = disk_reconcile_state.activation_guard().await;
    let game_lock = disk_reconcile_state.game_lock(&request.game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = operation_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
        )
        .await?;
    let mut result = crate::modules::reconciliation::application::disk_reconcile::source_recovery::apply_game_mods_directory(
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: pool.inner(),
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: operation_lock.inner().inner_lock(),
            progress_reporter: None,
        },
        request,
        &game_guard,
        operation_guard.op_guard(),
    )
    .await?;
    watcher.invalidate_session();
    *crate::shared::sync::lock(&watcher.watcher) = None;
    drop(operation_guard);
    drop(game_guard);
    if let Err(error) =
        crate::modules::workspace::application::scanner::watcher::lifecycle::start_watcher(
            app.clone(),
            watcher.inner(),
            pool.inner().clone(),
            result.game.mod_path.to_string_lossy().into_owned(),
            game_id,
        )
    {
        let warning = format!("The mods watcher could not restart: {error}");
        log::warn!("{warning}");
        result.watcher_warning = Some(warning);
    }
    if config.get_settings().active_game_id.as_deref() == Some(result.game.id.as_str()) {
        crate::modules::reconciliation::api::enqueue_runtime_sync(
            &app,
            pool.inner(),
            &result.game.id,
            crate::modules::reconciliation::api::RuntimeSyncCause::ModsRootChanged,
        );
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_onboarding_indexing_game_with_status(
    app: &tauri::AppHandle,
    session_id: &str,
    game_id: &str,
    pool: &sqlx::SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    watcher: &crate::modules::workspace::application::scanner::watcher::WatcherState,
    disk_reconcile_state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    operation_lock: &crate::modules::mutation::coordinator::MutationCoordinator,
    sessions: &crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    AppError,
> {
    let result = reconcile_onboarding_indexing_game_impl(
        app,
        session_id,
        game_id,
        pool,
        config,
        watcher,
        disk_reconcile_state,
        operation_lock,
        sessions,
    )
    .await;
    let phase = match &result {
        Ok(result) => background_phase_for_reconcile_status(&result.status),
        Err(_) => {
            crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::Failed
        }
    };
    if let Err(error) =
        update_onboarding_background_phase(app, sessions, session_id, game_id, phase)
    {
        log::debug!("Could not update onboarding background indexing status: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn reconcile_onboarding_indexing_game(
    app: tauri::AppHandle,
    session_id: String,
    game_id: String,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    operation_lock: State<'_, crate::modules::mutation::coordinator::MutationCoordinator>,
    sessions: State<'_, crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    AppError,
> {
    reconcile_onboarding_indexing_game_with_status(
        &app,
        &session_id,
        &game_id,
        pool.inner(),
        config.inner(),
        watcher.inner(),
        disk_reconcile_state.inner(),
        operation_lock.inner(),
        sessions.inner(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn continue_onboarding_indexing_in_background(
    app: tauri::AppHandle,
    session_id: String,
    game_ids: Vec<String>,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>,
) -> Result<(), AppError> {
    sessions.mark_background_started(&session_id, &game_ids)?;
    if let Err(error) = crate::modules::reconciliation::application::disk_reconcile::onboarding_recovery::replace_pending_game_ids(
        pool.inner(),
        &game_ids,
    )
    .await
    {
        let _ = sessions.cancel(&session_id);
        return Err(error);
    }
    let worker_app = app.clone();
    let worker_sessions = sessions.inner().clone();
    tokio::spawn(async move {
        for game_id in game_ids {
            let pool = worker_app.state::<sqlx::SqlitePool>();
            let config =
                worker_app.state::<crate::modules::settings::application::config::ConfigService>();
            let watcher = worker_app
                .state::<crate::modules::workspace::application::scanner::watcher::WatcherState>(
            );
            let disk_reconcile_state = worker_app.state::<
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
            >();
            let operation_lock =
                worker_app.state::<crate::modules::mutation::coordinator::MutationCoordinator>();
            match reconcile_onboarding_indexing_game_with_status(
                &worker_app,
                &session_id,
                &game_id,
                pool.inner(),
                config.inner(),
                watcher.inner(),
                disk_reconcile_state.inner(),
                operation_lock.inner(),
                &worker_sessions,
            )
            .await
            {
                Ok(result) if result.status.applied() => {
                    let pool = worker_app.state::<sqlx::SqlitePool>();
                    if let Err(error) = crate::modules::reconciliation::application::disk_reconcile::onboarding_recovery::remove_pending_game_id(
                        pool.inner(),
                        &game_id,
                    )
                    .await
                    {
                        log::warn!(
                            "Could not clear completed background onboarding game {game_id}: {error}"
                        );
                    }
                }
                Ok(result) => {
                    log::warn!(
                        "Background onboarding indexing needs attention for game {game_id}: {:?}",
                        result.status
                    );
                }
                Err(error) => {
                    log::warn!("Background onboarding indexing failed for game {game_id}: {error}");
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_onboarding_indexing_background_status(
    sessions: State<'_, crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>,
) -> Result<
    Vec<crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundStatus>,
    AppError,
>{
    Ok(sessions.background_statuses())
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn reconcile_disk_state_cmd(
    app: tauri::AppHandle,
    game_id: String,
    reason: crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason,
    changed_paths: Option<Vec<String>>,
    force_full: Option<bool>,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    operation_lock: State<'_, crate::modules::mutation::coordinator::MutationCoordinator>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    AppError,
> {
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    let started_at = std::time::Instant::now();
    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .inner()
        .clone();
    // Opening Mods can race the workspace query which starts initial recovery.
    // Reuse that single pass instead of queueing a second full scan behind it.
    if should_wait_for_initial_recovery(
        &reason,
        disk_reconcile_state.initial_recovery_readiness(&game_id),
    ) {
        return match crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_disk_recovery(
            &app,
            pool.inner(),
            disk_reconcile_state.inner(),
            &game_id,
        )
        .await
        {
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Completed(
                result,
            ) => Ok(*result),
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryOutcome::Failed(
                error,
            ) => Err(AppError::Io(error)),
        };
    }
    let progress_reporter = std::sync::Arc::new(
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
            app.clone(),
            game_id.clone(),
            reason.clone(),
        ),
    );
    let result = crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: pool.inner(),
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: operation_lock.inner().inner_lock(),
            progress_reporter: Some(progress_reporter),
        },
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
            game_id,
            reason,
            changed_paths.unwrap_or_default(),
            force_full.unwrap_or(false),
        )
        .defer_overlay_sync(),
    )
    .await;
    if let Ok(reconcile) = &result {
        // `reconcile_disk_state` has returned, so its game and operation
        // guards are gone before a reconstructible runtime refresh is queued.
        enqueue_runtime_sync_after_manual_reconcile(&app, pool.inner(), config.inner(), reconcile);
    }
    if diagnostics_enabled && result.is_ok() {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::Reconcile,
            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
        )
        .with_duration(started_at.elapsed());
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn begin_onboarding_indexing(
    app: tauri::AppHandle,
    game_ids: Vec<String>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    sessions: State<'_, crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSession,
    AppError,
> {
    let configured_games = config.get_settings().games;
    let requested_games = game_ids
        .iter()
        .map(|game_id| {
            configured_games
                .iter()
                .find(|game| game.id == *game_id)
                .cloned()
                .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}' not found")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let progress_app = app.clone();
    let progress_sessions = sessions.inner().clone();
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    let telemetry_sink = diagnostics_enabled
        .then(|| {
            app.try_state::<crate::modules::system::application::telemetry::TelemetrySink>()
                .map(|sink| sink.inner().clone())
        })
        .flatten();
    let phase_state = Arc::new(Mutex::new(HashMap::<
        String,
        OnboardingSnapshotTelemetryState,
    >::new()));
    let prepare_started_at = Instant::now();
    let progress_telemetry_sink = telemetry_sink.clone();
    let progress_phase_state = Arc::clone(&phase_state);
    let result = sessions
        .begin_with_progress(requested_games, move |progress| {
            if let Some(status) = progress_sessions.record_snapshot_progress(&progress) {
                emit_onboarding_background_status(&progress_app, status);
            }
            if let Some(sink) = &progress_telemetry_sink {
                let event = {
                    let mut state = crate::shared::sync::lock(&progress_phase_state);
                    let game_state = state.entry(progress.game_id.clone()).or_default();
                    match progress.phase {
                        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotPhase::Metadata
                            if game_state.preparation_started_at.is_none() => {
                                game_state.preparation_started_at = Some(Instant::now());
                                None
                            }
                        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotPhase::Ready => {
                            game_state.preparation_started_at.take().map(|started_at| {
                                crate::modules::system::application::telemetry::TelemetryEvent::new(
                                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingPreparation,
                                    crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                                    crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                                )
                                .with_duration(started_at.elapsed())
                            })
                        }
                        _ => None,
                    }
                };
                if let Some(event) = event {
                    sink.try_enqueue([event]);
                }
            }
            if let Err(error) = progress_app.emit("onboarding_indexing:snapshot_progress", progress)
            {
                log::debug!("Could not emit onboarding snapshot progress: {error}");
            }
        })
        .await;
    if let (Some(sink), Err(error)) = (&telemetry_sink, &result) {
        let duration = {
            let state = crate::shared::sync::lock(&phase_state);
            state
                .values()
                .filter_map(|game_state| game_state.preparation_started_at)
                .max()
                .map(|started_at| started_at.elapsed())
                .unwrap_or_else(|| prepare_started_at.elapsed())
        };
        sink.try_enqueue([
            crate::modules::system::application::telemetry::TelemetryEvent::new(
                crate::modules::system::application::telemetry::TelemetryOperation::OnboardingPreparation,
                telemetry_outcome_for_error(error),
                crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(
                    error,
                ),
            )
            .with_duration(duration),
        ]);
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_onboarding_indexing_game_impl(
    app: &tauri::AppHandle,
    session_id: &str,
    game_id: &str,
    pool: &sqlx::SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    watcher: &crate::modules::workspace::application::scanner::watcher::WatcherState,
    disk_reconcile_state: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    operation_lock: &crate::modules::mutation::coordinator::MutationCoordinator,
    sessions: &crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    AppError,
> {
    use crate::modules::reconciliation::application::disk_reconcile::onboarding_session::ConsumedOnboardingSnapshot;
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
        DiskReconcileContext, DiskReconcileProgressReporter, DiskReconcileRequest,
    };
    use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason;

    let telemetry_sink = config
        .get_settings()
        .diagnostics
        .telemetry_enabled
        .then(|| {
            app.try_state::<crate::modules::system::application::telemetry::TelemetrySink>()
                .map(|sink| sink.inner().clone())
        })
        .flatten();
    let snapshot_lease = match sessions.consume(session_id, game_id).await? {
        ConsumedOnboardingSnapshot::Snapshot(lease) => Some(lease),
        ConsumedOnboardingSnapshot::FullFallback => None,
    };
    update_onboarding_background_phase(
        app,
        sessions,
        session_id,
        game_id,
        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingBackgroundPhase::Applying,
    )?;
    let initial_recheck = snapshot_lease.is_none();
    if initial_recheck {
        app.emit(
            "onboarding_indexing:snapshot_progress",
            snapshot_rechecking_progress(session_id.to_string(), game_id.to_string()),
        )?;
    }
    let progress_reporter = std::sync::Arc::new(DiskReconcileProgressReporter::new(
        app.clone(),
        game_id.to_string(),
        DiskReconcileReason::OnboardingCompleted,
    ));
    let context = DiskReconcileContext {
        pool,
        config,
        state: disk_reconcile_state,
        watcher_suppressor: watcher.suppressor.clone(),
        operation_lock: operation_lock.inner_lock(),
        progress_reporter: Some(progress_reporter),
    };
    let mut request = DiskReconcileRequest::manual(
        game_id.to_string(),
        DiskReconcileReason::OnboardingCompleted,
        Vec::new(),
        true,
    );
    if let Some(lease) = &snapshot_lease {
        request = request.with_precomputed_discovery(lease.discovery());
    }
    let apply_started_at = Instant::now();
    let mut result = match crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
        context.clone(),
        request,
    )
    .await
    {
        Ok(result) => {
            record_onboarding_telemetry(
                &telemetry_sink,
                if initial_recheck {
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingRecheck
                } else {
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingApply
                },
                crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                apply_started_at.elapsed(),
            );
            result
        }
        Err(error) => {
            record_onboarding_telemetry(
                &telemetry_sink,
                if initial_recheck {
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingRecheck
                } else {
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingApply
                },
                telemetry_outcome_for_error(&error),
                crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(&error),
                apply_started_at.elapsed(),
            );
            return Err(error);
        }
    };

    // The watcher stayed alive until the transaction completed. A late event
    // invalidates the preflight snapshot, so settle with the generic full path.
    let changed_during_apply = match snapshot_lease.as_ref() {
        Some(lease) => lease.observed_changes_during_apply().await,
        None => false,
    };
    if changed_during_apply {
        app.emit(
            "onboarding_indexing:snapshot_progress",
            snapshot_rechecking_progress(session_id.to_string(), game_id.to_string()),
        )?;
        let recheck_started_at = Instant::now();
        result = match crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
            context,
            DiskReconcileRequest::manual(
                game_id.to_string(),
                DiskReconcileReason::OnboardingCompleted,
                Vec::new(),
                true,
            ),
        )
        .await
        {
            Ok(result) => {
                record_onboarding_telemetry(
                    &telemetry_sink,
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingRecheck,
                    crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                    crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                    recheck_started_at.elapsed(),
                );
                result
            }
            Err(error) => {
                record_onboarding_telemetry(
                    &telemetry_sink,
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingRecheck,
                    telemetry_outcome_for_error(&error),
                    crate::modules::system::application::telemetry::TelemetryErrorCode::from_app_error(&error),
                    recheck_started_at.elapsed(),
                );
                return Err(error);
            }
        };
    }
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_onboarding_indexing(
    session_id: String,
    sessions: State<'_, crate::modules::reconciliation::application::disk_reconcile::onboarding_session::OnboardingIndexingSessionStore>,
) -> Result<(), AppError> {
    sessions.cancel(&session_id)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn resolve_rename_confirmations(
    game_id: String,
    resolutions: Vec<crate::modules::reconciliation::application::disk_reconcile::types::RenameConfirmationResolution>,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    operation_lock: State<'_, crate::modules::mutation::coordinator::MutationCoordinator>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    AppError,
> {
    if resolutions.is_empty() {
        return Err(AppError::Validation(
            "At least one rename confirmation resolution is required".to_string(),
        ));
    }
    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| AppError::Validation(format!("Game '{game_id}' was not found")))?;
    let events = resolutions
        .into_iter()
        .map(|resolution| {
            let apply_as_rename = matches!(
                resolution.action,
                crate::modules::reconciliation::application::disk_reconcile::types::RenameConfirmationResolutionAction::Rename
            );
            let (from, to) = if apply_as_rename {
                let previous = resolution.previous_path.as_deref().ok_or_else(|| {
                    AppError::Validation(
                        "Confirmed rename requires a previous folder path".to_string(),
                    )
                })?;
                let current = resolution.current_path.as_deref().ok_or_else(|| {
                    AppError::Validation(
                        "Confirmed rename requires a current folder path".to_string(),
                    )
                })?;
                (
                    Some(checked_resolution_path(&game.mod_path, previous)?),
                    Some(checked_resolution_path(&game.mod_path, current)?),
                )
            } else {
                (None, None)
            };
            Ok(
                crate::modules::workspace::application::scanner::watcher::ModWatchEvent::RenameResolution {
                    group_id: resolution.group_id,
                    from,
                    to,
                    apply_as_rename,
                },
            )
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: pool.inner(),
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: operation_lock.inner().inner_lock(),
            progress_reporter: None,
        },
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::rename_resolutions(
            game_id, events,
        ),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        background_phase_for_reconcile_status, checked_resolution_path,
        manual_reconcile_runtime_request, should_wait_for_initial_recovery,
    };
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        DiskReconcileReason, DiskReconcileStatus, OnboardingIndexingBackgroundPhase,
    };
    use crate::modules::system::application::app::post_apply::RuntimeSyncRequest;

    #[test]
    fn rename_confirmation_paths_must_be_relative_and_contained() {
        let root = std::path::Path::new("E:/Mods");
        assert!(checked_resolution_path(root, "Alice/Blue").is_ok());
        assert!(checked_resolution_path(root, "../Outside").is_err());
        assert!(checked_resolution_path(root, "E:/Outside").is_err());
        assert!(checked_resolution_path(root, "Alice/../Outside").is_err());
    }

    #[test]
    fn mods_entry_reuses_an_in_flight_initial_recovery() {
        assert!(should_wait_for_initial_recovery(
            &DiskReconcileReason::ModsViewEntered,
            InitialRecoveryReadiness::Syncing { generation: 1 },
        ));
        assert!(!should_wait_for_initial_recovery(
            &DiskReconcileReason::WatcherBatch,
            InitialRecoveryReadiness::Syncing { generation: 1 },
        ));
    }

    #[test]
    fn manual_reconcile_keeps_known_roots_scoped_for_runtime() {
        assert!(matches!(
            manual_reconcile_runtime_request(&["Character/Mod A".to_string()]),
            RuntimeSyncRequest::ScopedRoots { .. }
        ));
        assert!(matches!(
            manual_reconcile_runtime_request(&[]),
            RuntimeSyncRequest::Full
        ));
    }

    #[test]
    fn onboarding_background_ready_requires_an_applied_reconcile_status() {
        for status in [
            DiskReconcileStatus::Applied,
            DiskReconcileStatus::AppliedWithFolderConflicts,
        ] {
            assert_eq!(
                background_phase_for_reconcile_status(&status),
                OnboardingIndexingBackgroundPhase::Ready
            );
        }
        for status in [
            DiskReconcileStatus::SourceUnavailable,
            DiskReconcileStatus::NeedsRenameConfirmation,
        ] {
            assert_eq!(
                background_phase_for_reconcile_status(&status),
                OnboardingIndexingBackgroundPhase::NeedsAttention
            );
        }
    }
}
