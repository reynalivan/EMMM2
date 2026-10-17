use tauri::{Emitter, Manager, State};

static RUNTIME_ENQUEUE_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));
static RUNTIME_WORK_PERMITS: std::sync::LazyLock<tokio::sync::Semaphore> =
    std::sync::LazyLock::new(|| tokio::sync::Semaphore::new(2));

struct RuntimeSyncAdmission {
    cause: RuntimeSyncCause,
    activation_authority: Option<ActivationAuthority>,
    request: RuntimeSyncRequest,
    reply: Option<tokio::sync::oneshot::Sender<RuntimeSyncReply>>,
    expected_generation: Option<u64>,
}

use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
    ActivationAuthority, DiskReconcileState, RuntimeSyncReply, StagedRuntimeEffects,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    PendingRuntimeEffects, RuntimeSyncPhase, RuntimeSyncReloadStatus, RuntimeSyncStatus,
};
use crate::modules::system::application::app::post_apply::{
    OverlaySyncCause, RuntimeModChange, RuntimeModOutcome, RuntimeReloadOutcome,
    RuntimeSyncRequest, RuntimeSyncResult,
};

#[derive(Debug, Clone, Copy)]
pub enum RuntimeSyncCause {
    EffectiveModsChanged,
    GameActivated,
    ModsRootChanged,
    ImporterRootChanged,
    SafeModeChanged,
    CollectionApplied,
    SettingsChanged,
    Recovery,
    ManualRetry,
}

impl RuntimeSyncCause {
    fn label(self) -> &'static str {
        match self {
            Self::EffectiveModsChanged => "effective_mods_changed",
            Self::GameActivated => "game_activated",
            Self::ModsRootChanged => "mods_root_changed",
            Self::ImporterRootChanged => "importer_root_changed",
            Self::SafeModeChanged => "safe_mode_changed",
            Self::CollectionApplied => "collection_applied",
            Self::SettingsChanged => "settings_changed",
            Self::Recovery => "recovery",
            Self::ManualRetry => "manual_retry",
        }
    }

    fn overlay_cause(self) -> OverlaySyncCause {
        match self {
            Self::EffectiveModsChanged => OverlaySyncCause::EffectiveModsChanged,
            Self::GameActivated => OverlaySyncCause::GameActivated,
            Self::ModsRootChanged => OverlaySyncCause::ModsRootChanged,
            Self::ImporterRootChanged => OverlaySyncCause::ImporterRootChanged,
            Self::SafeModeChanged => OverlaySyncCause::SafeModeChanged,
            Self::CollectionApplied => OverlaySyncCause::CollectionApplied,
            Self::SettingsChanged => OverlaySyncCause::SettingsChanged,
            Self::Recovery => OverlaySyncCause::Recovery,
            Self::ManualRetry => OverlaySyncCause::Recovery,
        }
    }
}

#[specta::specta]
#[tauri::command]
pub fn retry_runtime_sync(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    disk_reconcile_state: State<'_, DiskReconcileState>,
    game_id: String,
) -> Result<u64, crate::shared::errors::AppError> {
    if config.get_settings().active_game_id.as_deref() != Some(game_id.as_str()) {
        return Err(crate::shared::errors::AppError::Validation(
            "Runtime sync can only be retried for the active game".to_string(),
        ));
    }
    ensure_runtime_retry_ready(disk_reconcile_state.initial_recovery_readiness(&game_id))?;
    Ok(enqueue_runtime_sync(
        &app,
        pool.inner(),
        &game_id,
        RuntimeSyncCause::ManualRetry,
    ))
}

fn ensure_runtime_retry_ready(
    readiness: crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness,
) -> Result<(), crate::shared::errors::AppError> {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    match readiness {
        InitialRecoveryReadiness::Ready { .. } => Ok(()),
        InitialRecoveryReadiness::Unstarted { .. } | InitialRecoveryReadiness::Syncing { .. } => {
            Err(crate::shared::errors::AppError::Validation(
                "Game activation is still syncing; retry runtime sync after it is ready"
                    .to_string(),
            ))
        }
        InitialRecoveryReadiness::Failed { .. } => {
            Err(crate::shared::errors::AppError::Validation(
                "Game activation failed; retry activation before runtime sync".to_string(),
            ))
        }
    }
}

pub fn enqueue_runtime_sync(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    cause: RuntimeSyncCause,
) -> u64 {
    let activation_authority = app
        .state::<DiskReconcileState>()
        .activation_authority_for(game_id);
    enqueue_runtime_sync_inner(
        app,
        pool,
        game_id,
        cause,
        activation_authority,
        RuntimeSyncRequest::Full,
    )
}

/// Synchronous callers await the existing worker's owned result, without a
/// separate publication path or an additional scheduler.
pub async fn request_runtime_sync_and_wait(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    cause: RuntimeSyncCause,
) -> Result<RuntimeSyncResult, crate::shared::errors::AppError> {
    let authority = app
        .state::<DiskReconcileState>()
        .activation_authority_for(game_id);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    enqueue_runtime_sync_inner_with_reply(
        app,
        pool,
        game_id,
        RuntimeSyncAdmission {
            cause,
            activation_authority: authority,
            request: RuntimeSyncRequest::Full,
            reply: Some(sender),
            expected_generation: None,
        },
    );
    match receiver.await {
        Ok(RuntimeSyncReply::Completed(result)) => result,
        Ok(RuntimeSyncReply::Superseded) => Ok(crate::modules::system::api::app::post_apply::superseded_sync_result(cause.overlay_cause())),
        Err(_) => Err(crate::shared::errors::AppError::Internal("Runtime worker stopped before returning its publication result; pending work is retained".to_string())),
    }
}

pub fn enqueue_runtime_sync_scoped(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    cause: RuntimeSyncCause,
    request: RuntimeSyncRequest,
) -> u64 {
    let activation_authority = app
        .state::<DiskReconcileState>()
        .activation_authority_for(game_id);
    enqueue_runtime_sync_inner(app, pool, game_id, cause, activation_authority, request)
}

pub fn runtime_sync_request_for_roots(roots: &[String]) -> RuntimeSyncRequest {
    RuntimeSyncRequest::ScopedRoots {
        roots: roots
            .iter()
            .cloned()
            .map(crate::modules::system::domain::mod_path::ModFolderPath::from_stored)
            .collect(),
    }
}

/// Prefer stable mod IDs for leaf toggles. Parent switches intentionally fall
/// back to a compact root scope because every descendant's effective state can
/// change even though only the parent directory was renamed.
pub async fn runtime_sync_request_for_changed_paths(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_root: &std::path::Path,
    changed_paths: &[String],
    fallback_roots: &[String],
) -> RuntimeSyncRequest {
    let unique_path_count = changed_paths
        .iter()
        .map(|path| {
            crate::shared::path_key::folder_path_key(path, Some(&mods_root.to_string_lossy()))
        })
        .collect::<std::collections::HashSet<_>>()
        .len();
    let rows =
        match crate::modules::library::adapters::sqlite::mods::get_exact_runtime_mods_for_paths(
            pool,
            game_id,
            mods_root,
            changed_paths,
        )
        .await
        {
            Ok(rows) => rows,
            Err(error) => {
                log::warn!(
                "Could not resolve leaf runtime scope for game '{game_id}'; using root scope: {error}"
            );
                return runtime_sync_request_for_roots(fallback_roots);
            }
        };
    if rows.len() != unique_path_count || rows.iter().any(|row| row.has_descendants != 0) {
        return runtime_sync_request_for_roots(fallback_roots);
    }

    RuntimeSyncRequest::Scoped {
        changes: rows
            .into_iter()
            .map(|row| RuntimeModChange {
                mod_id: row.id,
                folder_path: crate::modules::system::domain::mod_path::ModFolderPath::from_stored(
                    row.folder_path,
                ),
                outcome: if row.status == 1 {
                    RuntimeModOutcome::Enabled
                } else {
                    RuntimeModOutcome::Disabled
                },
            })
            .collect(),
    }
}

/// Reserve the committed root scope synchronously while the mutation lease is
/// held. Runtime DB reads/harvesting belong to the worker, not this boundary.
/// Empty rewrites queue a status-only generation.
pub fn enqueue_runtime_sync_for_rewrites(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_root: &std::path::Path,
    cause: RuntimeSyncCause,
    rewrites: &[crate::modules::workspace::domain::workspace::WorkspacePathRewrite],
) -> u64 {
    let request = runtime_sync_request_for_rewrites(mods_root, rewrites);
    enqueue_runtime_sync_scoped(app, pool, game_id, cause, request)
}

fn runtime_sync_request_for_rewrites(
    mods_root: &std::path::Path,
    rewrites: &[crate::modules::workspace::domain::workspace::WorkspacePathRewrite],
) -> RuntimeSyncRequest {
    let roots = rewrites
        .iter()
        .flat_map(|rewrite| [&rewrite.old_path, &rewrite.new_path])
        .map(|path| crate::shared::path_key::relative_to_root(path, mods_root))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    runtime_sync_request_for_roots(&roots)
}

pub(crate) fn enqueue_runtime_sync_with_authority(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    cause: RuntimeSyncCause,
    activation_authority: ActivationAuthority,
) -> u64 {
    enqueue_runtime_sync_inner(
        app,
        pool,
        game_id,
        cause,
        Some(activation_authority),
        RuntimeSyncRequest::Full,
    )
}

fn enqueue_runtime_sync_inner(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    cause: RuntimeSyncCause,
    activation_authority: Option<ActivationAuthority>,
    request: RuntimeSyncRequest,
) -> u64 {
    enqueue_runtime_sync_inner_with_reply(
        app,
        pool,
        game_id,
        RuntimeSyncAdmission {
            cause,
            activation_authority,
            request,
            reply: None,
            expected_generation: None,
        },
    )
    .expect("unconditional runtime admission cannot be superseded")
}

fn enqueue_runtime_sync_inner_with_reply(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    admission: RuntimeSyncAdmission,
) -> Option<u64> {
    let RuntimeSyncAdmission {
        cause,
        activation_authority,
        request,
        reply,
        expected_generation,
    } = admission;
    let state = app.state::<DiskReconcileState>();
    let _enqueue_guard = crate::shared::sync::lock(&RUNTIME_ENQUEUE_LOCK);
    if expected_generation
        .is_some_and(|generation| !state.runtime_sync_is_current(game_id, generation))
    {
        return None;
    }
    // Admission retains intent even if the caller drops its receiver or the
    // worker stops before claiming this request. Only the winning worker ACKs.
    state.stage_runtime_effects(
        game_id,
        PendingRuntimeEffects {
            collections_dirty: true,
            overlay_refresh: true,
        },
    );
    let publication_revision = crate::modules::system::application::app::post_apply::reserve_overlay_sync_revision_for_cause(
            game_id,
            request,
            cause.overlay_cause(),
        )
        .map_err(|error| {
            log::error!("Could not reserve runtime publication for '{game_id}': {error}");
            error
        })
        .ok();
    let enqueue = state.enqueue_runtime_sync_with_reply(
        game_id,
        cause.label(),
        publication_revision,
        activation_authority,
        reply,
    );
    drop(_enqueue_guard);
    emit_status(
        app,
        RuntimeSyncStatus {
            game_id: game_id.to_string(),
            generation: enqueue.generation,
            phase: RuntimeSyncPhase::Queued,
            cause: cause.label().to_string(),
            message: None,
            reload_status: None,
            reload_binding: None,
        },
    );
    if enqueue.start_worker {
        let app = app.clone();
        let pool = pool.clone();
        let game_id = game_id.to_string();
        tauri::async_runtime::spawn(async move {
            run_worker(app, pool, game_id, cause).await;
        });
    }
    Some(enqueue.generation)
}

fn finish_runtime_sync_generation(
    state: &DiskReconcileState,
    game_id: &str,
    generation: u64,
    staged: StagedRuntimeEffects,
    settled: bool,
) -> bool {
    // Pair the authority check, acknowledgement, and worker finish with
    // enqueue's lock. A generation superseded during publication must leave
    // the merged runtime effects for the newer queued job.
    let _enqueue_guard = crate::shared::sync::lock(&RUNTIME_ENQUEUE_LOCK);
    if settled && state.runtime_sync_is_current(game_id, generation) {
        state.acknowledge_runtime_effects(game_id, staged);
    }
    state.finish_runtime_sync(game_id, generation)
}

async fn run_worker(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
    initial_cause: RuntimeSyncCause,
) {
    let mut fallback_cause = initial_cause;
    loop {
        let state = app.state::<DiskReconcileState>();
        let Some(mut job) = state.claim_latest_runtime_sync(&game_id) else {
            log::error!("Runtime sync worker for '{game_id}' had no queued generation");
            return;
        };
        let cause = cause_from_label(&job.cause).unwrap_or(fallback_cause);
        fallback_cause = cause;
        let queued_work = std::time::Instant::now();
        let _work_permit = RUNTIME_WORK_PERMITS
            .acquire()
            .await
            .expect("runtime work semaphore is never closed");
        if !state.runtime_sync_is_current(&game_id, job.generation) {
            if let Some(reply) = job.reply.take() {
                let _ = reply.send(RuntimeSyncReply::Superseded);
            }
            let _ = state.finish_runtime_sync(&game_id, job.generation);
            continue;
        }
        let worker_started = std::time::Instant::now();
        log::debug!("RuntimeSync game_id={game_id} generation={} publication_revision={:?} permit_wait_ms={}", job.generation, job.publication_revision, queued_work.elapsed().as_millis());
        emit_status(
            &app,
            RuntimeSyncStatus {
                game_id: game_id.clone(),
                generation: job.generation,
                phase: RuntimeSyncPhase::Running,
                cause: job.cause.clone(),
                message: None,
                reload_status: None,
                reload_binding: None,
            },
        );

        let staged = state.stage_runtime_effects_for_settlement(
            &game_id,
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: true,
            },
        );
        let config = app.state::<crate::modules::settings::application::config::ConfigService>();
        let outcome = match job.publication_revision {
            Some(revision) => crate::modules::system::application::app::post_apply::request_overlay_sync_with_retry_for_game_at_revision(
                &pool,
                config.inner(),
                &game_id,
                cause.overlay_cause(),
                revision,
                job.activation_authority.clone(),
            )
            .await,
            None => Err(crate::shared::errors::AppError::Internal(
                "Runtime publication authority is unavailable; restart the application"
                    .to_string(),
            )),
        };
        let reply_result = outcome.clone();
        let Some((phase, message, settled, reload_status, reload_binding)) =
            runtime_sync_completion(outcome)
        else {
            if state.runtime_sync_is_current(&game_id, job.generation)
                && job
                    .activation_authority
                    .as_ref()
                    .is_some_and(|authority| authority.with_current(|| ()).is_some())
                && config.get_settings().active_game_id.as_deref() == Some(game_id.as_str())
            {
                // Legacy core-only callers can still reserve a revision without
                // queue admission. Recover their merged intent through this owner.
                // Check under admission's lock too: a real successor admitted
                // since the earlier check must retain its scope and waiter.
                enqueue_runtime_sync_inner_with_reply(
                    &app,
                    &pool,
                    &game_id,
                    RuntimeSyncAdmission {
                        cause: RuntimeSyncCause::Recovery,
                        activation_authority: job.activation_authority.clone(),
                        request: RuntimeSyncRequest::Full,
                        reply: None,
                        expected_generation: Some(job.generation),
                    },
                );
            }
            if let Some(reply) = job.reply.take() {
                let _ = reply.send(RuntimeSyncReply::Superseded);
            }
            if finish_runtime_sync_generation(&state, &game_id, job.generation, staged, false) {
                emit_status(
                    &app,
                    RuntimeSyncStatus {
                        game_id: game_id.clone(),
                        generation: job.generation,
                        phase: RuntimeSyncPhase::Failed,
                        cause: job.cause,
                        message: Some("Runtime synchronization lost its activation authority; pending work is retained. Retry after game activation is ready".to_string()),
                        reload_status: None,
                        reload_binding: None,
                    },
                );
                return;
            }
            continue;
        };
        if phase == RuntimeSyncPhase::Failed {
            log::error!(
                "Runtime sync failed game_id={} generation={} cause={} message={}",
                game_id,
                job.generation,
                job.cause,
                message.as_deref().unwrap_or("unknown runtime failure"),
            );
        }
        let authoritative =
            finish_runtime_sync_generation(&state, &game_id, job.generation, staged, settled);
        if let Some(reply) = job.reply.take() {
            let outcome = if authoritative {
                RuntimeSyncReply::Completed(reply_result)
            } else {
                RuntimeSyncReply::Superseded
            };
            let _ = reply.send(outcome);
        }
        log::debug!("RuntimeSync game_id={game_id} generation={} publication_revision={:?} phase={phase:?} authoritative={authoritative} elapsed_ms={}", job.generation, job.publication_revision, worker_started.elapsed().as_millis());
        if authoritative {
            emit_status(
                &app,
                RuntimeSyncStatus {
                    game_id: game_id.clone(),
                    generation: job.generation,
                    phase,
                    cause: job.cause,
                    message,
                    reload_status,
                    reload_binding,
                },
            );
            return;
        }
    }
}

type RuntimeSyncCompletion = (
    RuntimeSyncPhase,
    Option<String>,
    bool,
    Option<RuntimeSyncReloadStatus>,
    Option<String>,
);

fn runtime_sync_completion(
    outcome: Result<RuntimeSyncResult, crate::shared::errors::AppError>,
) -> Option<RuntimeSyncCompletion> {
    if outcome.as_ref().is_ok_and(RuntimeSyncResult::is_superseded) {
        return None;
    }
    Some(match outcome {
        Ok(result) if result.failure.is_some() => {
            (RuntimeSyncPhase::Failed, result.failure, false, None, None)
        }
        Ok(result) => match result.reload {
            RuntimeReloadOutcome::NeedsManualReload { binding, reason } => (
                RuntimeSyncPhase::NeedsManualReload,
                Some(reason),
                true,
                Some(RuntimeSyncReloadStatus::Manual),
                binding,
            ),
            RuntimeReloadOutcome::NotRequired => (
                RuntimeSyncPhase::Succeeded,
                None,
                true,
                Some(RuntimeSyncReloadStatus::NotRequired),
                None,
            ),
            RuntimeReloadOutcome::ReloadSent { binding } => (
                RuntimeSyncPhase::Succeeded,
                None,
                true,
                Some(RuntimeSyncReloadStatus::Sent),
                Some(binding),
            ),
        },
        Err(error) => (
            RuntimeSyncPhase::Failed,
            Some(error.to_string()),
            false,
            None,
            None,
        ),
    })
}

fn cause_from_label(label: &str) -> Option<RuntimeSyncCause> {
    match label {
        "effective_mods_changed" => Some(RuntimeSyncCause::EffectiveModsChanged),
        "game_activated" => Some(RuntimeSyncCause::GameActivated),
        "mods_root_changed" => Some(RuntimeSyncCause::ModsRootChanged),
        "importer_root_changed" => Some(RuntimeSyncCause::ImporterRootChanged),
        "safe_mode_changed" => Some(RuntimeSyncCause::SafeModeChanged),
        "collection_applied" => Some(RuntimeSyncCause::CollectionApplied),
        "settings_changed" => Some(RuntimeSyncCause::SettingsChanged),
        "recovery" => Some(RuntimeSyncCause::Recovery),
        "manual_retry" => Some(RuntimeSyncCause::ManualRetry),
        _ => None,
    }
}

fn emit_status(app: &tauri::AppHandle, status: RuntimeSyncStatus) {
    crate::modules::automation::api::update_hotkey_notice_runtime(app, &status);
    if let Err(error) = app.emit("runtime_sync:status", status) {
        log::warn!("Could not emit runtime sync status: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::errors::AppError;

    #[test]
    fn legitimate_skip_failure_and_manual_reload_keep_their_completion_contracts() {
        use crate::modules::system::api::app::post_apply::RuntimeSyncPublication;
        let mut result = RuntimeSyncResult {
            cause: OverlaySyncCause::SettingsChanged,
            publication: RuntimeSyncPublication::Skipped,
            reload: RuntimeReloadOutcome::NotRequired,
            failure: None,
        };
        assert_eq!(
            runtime_sync_completion(Ok(result.clone())),
            Some((
                RuntimeSyncPhase::Succeeded,
                None,
                true,
                Some(RuntimeSyncReloadStatus::NotRequired),
                None,
            ))
        );
        result.publication = RuntimeSyncPublication::Superseded;
        assert!(runtime_sync_completion(Ok(result.clone())).is_none());
        assert!(result.diagnostic_message().unwrap().contains("superseded"));
        result.publication = RuntimeSyncPublication::Published;
        result.reload = RuntimeReloadOutcome::NeedsManualReload {
            binding: Some("F10".into()),
            reason: "Focus the game and reload".into(),
        };
        assert_eq!(
            runtime_sync_completion(Ok(result.clone())),
            Some((
                RuntimeSyncPhase::NeedsManualReload,
                Some("Focus the game and reload".into()),
                true,
                Some(RuntimeSyncReloadStatus::Manual),
                Some("F10".into()),
            ))
        );
        result.reload = RuntimeReloadOutcome::ReloadSent {
            binding: "Ctrl+F10".into(),
        };
        assert_eq!(
            runtime_sync_completion(Ok(result.clone())),
            Some((
                RuntimeSyncPhase::Succeeded,
                None,
                true,
                Some(RuntimeSyncReloadStatus::Sent),
                Some("Ctrl+F10".into()),
            ))
        );
        result.publication = RuntimeSyncPublication::FailedBeforePublish;
        result.failure = Some("Publication is pending".into());
        assert_eq!(
            runtime_sync_completion(Ok(result)),
            Some((
                RuntimeSyncPhase::Failed,
                Some("Publication is pending".into()),
                false,
                None,
                None,
            ))
        );
        assert_eq!(
            runtime_sync_completion(Err(AppError::Io("Unavailable".into()))),
            Some((
                RuntimeSyncPhase::Failed,
                Some(AppError::Io("Unavailable".into()).to_string()),
                false,
                None,
                None,
            ))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn revision_only_supersession_cannot_acknowledge_or_report_success() {
        use crate::modules::system::application::app::post_apply;
        let _epoch = crate::modules::reconciliation::application::disk_reconcile::orchestrator::activation_epoch_test_guard_async().await;
        let state = DiskReconcileState::new();
        let game = format!("revision-only-{}", uuid::Uuid::new_v4());
        state.begin_activation(Some(game.clone()));
        let authority = state.activation_authority_for(&game);
        let older = post_apply::reserve_overlay_sync_revision(&game).unwrap();
        let queued =
            state.enqueue_runtime_sync(&game, "settings_changed", Some(older), authority.clone());
        let job = state.claim_latest_runtime_sync(&game).unwrap();
        let staged = state.stage_runtime_effects_for_settlement(
            &game,
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: true,
            },
        );
        let newer = post_apply::reserve_overlay_sync_revision(&game).unwrap();
        assert!(newer > older);
        assert!(state.runtime_sync_is_current(&game, queued.generation));
        let pool = sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap();
        let config = crate::modules::settings::application::config::ConfigService::new_for_test(
            pool.clone(),
        );
        let result = post_apply::request_overlay_sync_with_retry_for_game_at_revision(
            &pool,
            &config,
            &game,
            OverlaySyncCause::SettingsChanged,
            older,
            authority,
        )
        .await
        .unwrap();
        let completion = runtime_sync_completion(Ok(result));
        let settled = completion
            .as_ref()
            .is_some_and(|(_, _, settled, _, _)| *settled);
        finish_runtime_sync_generation(&state, &game, job.generation, staged, settled);
        let pending = state.stage_runtime_effects(&game, PendingRuntimeEffects::default());
        state.begin_activation(None);
        assert!(
            pending.collections_dirty && pending.overlay_refresh,
            "superseded work cannot acknowledge pending effects"
        );
        assert!(
            completion.is_none(),
            "superseded work cannot emit terminal success"
        );
    }

    #[test]
    fn runtime_sync_cause_round_trips_through_queue_label() {
        for cause in [
            RuntimeSyncCause::EffectiveModsChanged,
            RuntimeSyncCause::GameActivated,
            RuntimeSyncCause::ModsRootChanged,
            RuntimeSyncCause::ImporterRootChanged,
            RuntimeSyncCause::SafeModeChanged,
            RuntimeSyncCause::CollectionApplied,
            RuntimeSyncCause::SettingsChanged,
            RuntimeSyncCause::Recovery,
            RuntimeSyncCause::ManualRetry,
        ] {
            assert_eq!(
                cause_from_label(cause.label()).unwrap().label(),
                cause.label()
            );
        }
    }

    #[test]
    fn manual_retry_requires_completed_activation_recovery() {
        use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

        assert!(
            ensure_runtime_retry_ready(InitialRecoveryReadiness::Ready { generation: 4 }).is_ok()
        );
        assert!(
            ensure_runtime_retry_ready(InitialRecoveryReadiness::Syncing { generation: 4 })
                .is_err()
        );
        assert!(
            ensure_runtime_retry_ready(InitialRecoveryReadiness::Failed { generation: 4 }).is_err()
        );
    }

    #[test]
    fn committed_rewrites_reserve_runtime_without_waiting_on_database() {
        // The synchronous signature makes an awaited DB lookup at this
        // lease-held reservation boundary a compile error.
        let _: fn(
            &tauri::AppHandle,
            &sqlx::SqlitePool,
            &str,
            &std::path::Path,
            RuntimeSyncCause,
            &[crate::modules::workspace::domain::workspace::WorkspacePathRewrite],
        ) -> u64 = enqueue_runtime_sync_for_rewrites;
    }

    #[test]
    fn committed_rewrite_scope_keeps_old_and_new_roots_and_empty_status_updates() {
        use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;

        let root = std::path::Path::new("C:/Mods");
        let rewrites = vec![
            WorkspacePathRewrite {
                old_path: "C:/Mods/Parent/Skin".into(),
                new_path: "C:/Mods/DISABLED Parent/Skin".into(),
            },
            WorkspacePathRewrite {
                old_path: "C:/Mods/Parent/Skin".into(),
                new_path: "C:/Mods/DISABLED Parent/Skin".into(),
            },
        ];
        let RuntimeSyncRequest::ScopedRoots { roots } =
            runtime_sync_request_for_rewrites(root, &rewrites)
        else {
            panic!("rewrite reservation must use root scopes");
        };
        assert_eq!(
            roots
                .iter()
                .map(|root| root.as_stored())
                .collect::<Vec<_>>(),
            vec!["DISABLED Parent/Skin", "Parent/Skin"]
        );
        assert_eq!(
            runtime_sync_request_for_rewrites(root, &[]),
            RuntimeSyncRequest::ScopedRoots { roots: vec![] }
        );
    }

    #[test]
    fn superseded_generation_does_not_acknowledge_runtime_effects() {
        let state = DiskReconcileState::new();
        state.enqueue_runtime_sync("game", "first", Some(1), None);
        let first = state
            .claim_latest_runtime_sync("game")
            .expect("first generation should be claimable");
        let staged = state.stage_runtime_effects_for_settlement(
            "game",
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: true,
            },
        );
        state.enqueue_runtime_sync("game", "second", Some(2), None);

        assert!(!finish_runtime_sync_generation(
            &state,
            "game",
            first.generation,
            staged,
            true,
        ));
        let pending = state.stage_runtime_effects("game", PendingRuntimeEffects::default());
        assert!(pending.collections_dirty);
        assert!(pending.overlay_refresh);
    }
}
