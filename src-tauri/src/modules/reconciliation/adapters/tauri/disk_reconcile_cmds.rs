use crate::shared::errors::AppError;
use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason;

#[derive(Default)]
struct OnboardingSnapshotTelemetryState {
    metadata_recorded: bool,
    preparation_started_at: Option<Instant>,
    classification_started_at: Option<Instant>,
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
        files_inspected: 0,
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

    crate::modules::reconciliation::api::enqueue_runtime_sync(
        app,
        pool,
        &result.game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
    );
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
pub async fn plan_onboarding_indexing_work(
    game_ids: Vec<String>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<
    Vec<
        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingWorkPlan,
    >,
    AppError,
>{
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

    tokio::task::spawn_blocking(move || {
        crate::modules::reconciliation::application::disk_reconcile::work_plan::plan_onboarding_indexing_work(
            &requested_games,
        )
    })
    .await
    .map_err(|error| AppError::Internal(format!("Indexing work planning task failed: {error}")))?
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
            if let Some(sink) = &progress_telemetry_sink {
                let event = {
                    let mut state = crate::shared::sync::lock(&progress_phase_state);
                    let game_state = state.entry(progress.game_id.clone()).or_default();
                    match progress.phase {
                        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotPhase::Metadata
                            if progress.files_inspected == 0 => {
                                game_state.preparation_started_at = Some(Instant::now());
                                None
                            }
                        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotPhase::Classifying
                            if !game_state.metadata_recorded => {
                                game_state.metadata_recorded = true;
                                let preparation_duration = game_state
                                    .preparation_started_at
                                    .take()
                                    .map(|started_at| started_at.elapsed())
                                    .unwrap_or_else(|| Duration::from_millis(progress.elapsed_ms));
                                game_state.classification_started_at = Some(Instant::now());
                                Some(
                                    crate::modules::system::application::telemetry::TelemetryEvent::new(
                                        crate::modules::system::application::telemetry::TelemetryOperation::OnboardingPreparation,
                                        crate::modules::system::application::telemetry::TelemetryOutcome::Success,
                                        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
                                    )
                                    .with_duration(preparation_duration),
                                )
                            }
                        crate::modules::reconciliation::application::disk_reconcile::types::OnboardingIndexingSnapshotPhase::Ready => {
                            game_state.classification_started_at.take().map(|started_at| {
                                crate::modules::system::application::telemetry::TelemetryEvent::new(
                                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingClassification,
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
        let (operation, duration) = {
            let state = crate::shared::sync::lock(&phase_state);
            let classification_started_at = state
                .values()
                .filter_map(|game_state| game_state.classification_started_at)
                .max();
            let preparation_started_at = state
                .values()
                .filter_map(|game_state| game_state.preparation_started_at)
                .max();
            match (classification_started_at, preparation_started_at) {
                (Some(started_at), _) => (
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingClassification,
                    started_at.elapsed(),
                ),
                (None, Some(started_at)) => (
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingPreparation,
                    started_at.elapsed(),
                ),
                (None, None) => (
                    crate::modules::system::application::telemetry::TelemetryOperation::OnboardingPreparation,
                    prepare_started_at.elapsed(),
                ),
            }
        };
        sink.try_enqueue([
            crate::modules::system::application::telemetry::TelemetryEvent::new(
                operation,
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
    let snapshot_lease = match sessions.consume(&session_id, &game_id).await? {
        ConsumedOnboardingSnapshot::Snapshot(lease) => {
            if let Some(update) = lease.work_plan_update() {
                app.emit("onboarding_indexing:work_plan", update)?;
            }
            Some(lease)
        }
        ConsumedOnboardingSnapshot::FullFallback => None,
    };
    let initial_recheck = snapshot_lease.is_none();
    if initial_recheck {
        app.emit(
            "onboarding_indexing:snapshot_progress",
            snapshot_rechecking_progress(session_id.clone(), game_id.clone()),
        )?;
    }
    let progress_reporter = std::sync::Arc::new(DiskReconcileProgressReporter::new(
        app.clone(),
        game_id.clone(),
        DiskReconcileReason::OnboardingCompleted,
    ));
    let context = DiskReconcileContext {
        pool: pool.inner(),
        config: config.inner(),
        state: disk_reconcile_state.inner(),
        watcher_suppressor: watcher.suppressor.clone(),
        operation_lock: operation_lock.inner().inner_lock(),
        progress_reporter: Some(progress_reporter),
    };
    let mut request = DiskReconcileRequest::manual(
        game_id.clone(),
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
            snapshot_rechecking_progress(session_id, game_id.clone()),
        )?;
        let recheck_started_at = Instant::now();
        result = match crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
            context,
            DiskReconcileRequest::manual(
                game_id,
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
    use super::{checked_resolution_path, should_wait_for_initial_recovery};
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
    use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason;

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
}
