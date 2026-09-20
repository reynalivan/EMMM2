use tauri::{Emitter, Manager, State};

static RUNTIME_ENQUEUE_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));
static RUNTIME_WORK_PERMITS: std::sync::LazyLock<tokio::sync::Semaphore> =
    std::sync::LazyLock::new(|| tokio::sync::Semaphore::new(2));

use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
    ActivationAuthority, DiskReconcileState, StagedRuntimeEffects,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    PendingRuntimeEffects, RuntimeSyncPhase, RuntimeSyncStatus,
};
use crate::modules::system::application::app::post_apply::{
    OverlaySyncCause, RuntimeModChange, RuntimeModOutcome, RuntimeReloadOutcome, RuntimeSyncRequest,
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

/// Build and enqueue the narrowest runtime refresh for committed filesystem
/// rewrites. An empty rewrite set still queues a status-only generation so a
/// collection name or Safe Mode change reaches the published artifact.
pub async fn enqueue_runtime_sync_for_rewrites(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_root: &std::path::Path,
    cause: RuntimeSyncCause,
    rewrites: &[crate::modules::workspace::domain::workspace::WorkspacePathRewrite],
) -> u64 {
    let changed_paths = rewrites
        .iter()
        .map(|rewrite| rewrite.new_path.clone())
        .collect::<Vec<_>>();
    let fallback_roots = rewrites
        .iter()
        .map(|rewrite| crate::shared::path_key::relative_to_root(&rewrite.new_path, mods_root))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let request = runtime_sync_request_for_changed_paths(
        pool,
        game_id,
        mods_root,
        &changed_paths,
        &fallback_roots,
    )
    .await;
    enqueue_runtime_sync_scoped(app, pool, game_id, cause, request)
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
    let state = app.state::<DiskReconcileState>();
    let _enqueue_guard = crate::shared::sync::lock(&RUNTIME_ENQUEUE_LOCK);
    let publication_revision = crate::modules::system::application::app::post_apply::reserve_overlay_sync_revision_for_request(
            game_id,
            request,
        )
        .map_err(|error| {
            log::error!("Could not reserve runtime publication for '{game_id}': {error}");
            error
        })
        .ok();
    let enqueue = state.enqueue_runtime_sync(
        game_id,
        cause.label(),
        publication_revision,
        activation_authority,
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
    enqueue.generation
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
        let Some(job) = state.claim_latest_runtime_sync(&game_id) else {
            log::error!("Runtime sync worker for '{game_id}' had no queued generation");
            return;
        };
        let cause = cause_from_label(&job.cause).unwrap_or(fallback_cause);
        fallback_cause = cause;
        let _work_permit = RUNTIME_WORK_PERMITS
            .acquire()
            .await
            .expect("runtime work semaphore is never closed");
        if !state.runtime_sync_is_current(&game_id, job.generation) {
            let _ = state.finish_runtime_sync(&game_id, job.generation);
            continue;
        }
        emit_status(
            &app,
            RuntimeSyncStatus {
                game_id: game_id.clone(),
                generation: job.generation,
                phase: RuntimeSyncPhase::Running,
                cause: job.cause.clone(),
                message: None,
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
        let (phase, message, settled) = match outcome {
            Ok(result) if result.failure.is_some() => {
                (RuntimeSyncPhase::Failed, result.failure, false)
            }
            Ok(result) => match result.reload {
                RuntimeReloadOutcome::NeedsManualReload { reason, .. } => {
                    (RuntimeSyncPhase::NeedsManualReload, Some(reason), true)
                }
                RuntimeReloadOutcome::NotRequired | RuntimeReloadOutcome::ReloadSent { .. } => {
                    (RuntimeSyncPhase::Succeeded, None, true)
                }
            },
            Err(error) => (RuntimeSyncPhase::Failed, Some(error.to_string()), false),
        };
        let authoritative =
            finish_runtime_sync_generation(&state, &game_id, job.generation, staged, settled);
        if authoritative {
            emit_status(
                &app,
                RuntimeSyncStatus {
                    game_id: game_id.clone(),
                    generation: job.generation,
                    phase,
                    cause: job.cause,
                    message,
                },
            );
            return;
        }
    }
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
    if let Err(error) = app.emit("runtime_sync:status", status) {
        log::warn!("Could not emit runtime sync status: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
