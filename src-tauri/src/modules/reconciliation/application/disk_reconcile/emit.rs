//! Shared convergence hook for internal mutations.
//!
//! Runs a scoped `InternalMutation` Disk Reconcile and emits the result to
//! the frontend. Called after internal FS mutations so the runtime projection
//! converges with disk reality even if a manual DB sync step missed a case
//! or watcher events were dropped during suppression.

use crate::shared::errors::AppError;
use tauri::{Emitter, Manager};

use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
    reconcile_disk_state, DiskReconcileContext, DiskReconcileRequest, DiskReconcileState,
    InitialRecoveryClaim, InitialRecoveryOutcome,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    CommittedMutationSyncWarning, CommittedMutationSyncWarningKind, DiskReconcileReason,
    DiskReconcileResult, DiskReconcileStatus,
};

pub struct CommittedReconcileSettlement {
    pub reconcile: Option<DiskReconcileResult>,
    pub sync_warning: Option<CommittedMutationSyncWarning>,
}

/// Convert terminal convergence into data after the filesystem mutation has
/// committed. Returning an `Err` here would invite callers to retry an
/// irreversible rename/delete that already succeeded.
pub fn settle_committed_reconcile(
    outcome: Result<DiskReconcileResult, AppError>,
) -> CommittedReconcileSettlement {
    match outcome {
        Ok(result) if result.status.applied() => CommittedReconcileSettlement {
            reconcile: Some(result),
            sync_warning: None,
        },
        Ok(result) => {
            let message = result.error_message.clone().unwrap_or_else(|| {
                format!(
                    "Disk mutation succeeded but reconcile was blocked with status {:?}",
                    result.status
                )
            });
            CommittedReconcileSettlement {
                reconcile: Some(result),
                sync_warning: Some(CommittedMutationSyncWarning {
                    kind: CommittedMutationSyncWarningKind::ReconcileBlocked,
                    message,
                }),
            }
        }
        Err(error) => CommittedReconcileSettlement {
            reconcile: None,
            sync_warning: Some(CommittedMutationSyncWarning {
                kind: CommittedMutationSyncWarningKind::ReconcileFailed,
                message: format!("Disk mutation succeeded but projection sync failed: {error}"),
            }),
        },
    }
}

fn requires_user_resolution(status: &DiskReconcileStatus) -> bool {
    matches!(status, DiskReconcileStatus::NeedsRenameConfirmation)
}

fn initial_recovery_allows_mutation(
    readiness: crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness,
) -> bool {
    !matches!(
        readiness,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Syncing { .. }
    )
}

fn comparable_path(path: &str) -> String {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let without_verbatim_prefix = normalized
        .strip_prefix("//?/unc/")
        .map(|path| format!("//{path}"))
        .unwrap_or_else(|| {
            normalized
                .strip_prefix("//?/")
                .unwrap_or(&normalized)
                .to_string()
        });
    without_verbatim_prefix.trim_end_matches('/').to_string()
}

fn path_is_within_conflict_scope(path: &str, scope: &str) -> bool {
    let path = comparable_path(path);
    let scope = comparable_path(scope);
    path == scope
        || path.starts_with(&format!("{scope}/"))
        || scope.starts_with(&format!("{path}/"))
}

pub(crate) fn conflicts_intersect_paths(
    conflicts: &[crate::modules::reconciliation::application::disk_reconcile::types::FolderNameConflictGroup],
    paths: &[String],
) -> bool {
    paths.iter().any(|path| {
        conflicts.iter().any(|group| {
            group
                .candidates
                .iter()
                .any(|candidate| path_is_within_conflict_scope(path, &candidate.path))
        })
    })
}

/// Scoped `InternalMutation` reconcile without the frontend event. For flows
/// whose command RESULT already drives the frontend refresh (workspace
/// switch): emitting the event too would trigger a second full
/// invalidation+refetch round per toggle.
pub async fn run_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    run_internal_disk_reconcile_with_options(
        app,
        pool,
        game_id,
        changed_paths,
        DiskReconcileReason::InternalMutation,
        false,
        true,
        Vec::new(),
    )
    .await
}

pub async fn run_full_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    run_internal_disk_reconcile_with_options(
        app,
        pool,
        game_id,
        Vec::new(),
        DiskReconcileReason::InternalMutation,
        true,
        false,
        Vec::new(),
    )
    .await
}

/// Terminal full reconcile for a filesystem mutation that already owns the
/// per-game and operation locks. Re-entering the public orchestrator here
/// would deadlock on the game lock held by `lease`.
pub async fn run_full_internal_disk_reconcile_under_lease(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    lease: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    let config = app
        .try_state::<crate::modules::settings::application::config::ConfigService>()
        .ok_or_else(|| {
            AppError::Internal("ConfigService state missing for disk reconcile".to_string())
        })?;
    let watcher = app
        .try_state::<crate::modules::workspace::application::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState missing for disk reconcile".to_string()))?;
    let state = app.try_state::<DiskReconcileState>().ok_or_else(|| {
        AppError::Internal("DiskReconcileState missing for disk reconcile".to_string())
    })?;
    let operation_lock = app
        .try_state::<crate::platform::fs::operation_lock::OperationLock>()
        .ok_or_else(|| {
            AppError::Internal("OperationLock missing for disk reconcile".to_string())
        })?;
    let reason = DiskReconcileReason::InternalMutation;

    crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_under_lease(
        DiskReconcileContext {
            pool,
            config: config.inner(),
            state: state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: operation_lock.inner(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id,
                    reason.clone(),
                ),
            )),
        },
        DiskReconcileRequest::manual(game_id.to_string(), reason, Vec::new(), true),
        lease,
    )
    .await
}

/// Scoped terminal reconcile for a filesystem mutation that already owns the
/// per-game and operation locks. Path hints keep stable object/Collection
/// references attached to moved folders without reopening the public lock path.
pub async fn run_internal_disk_reconcile_with_path_hints_under_lease(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
    path_hints: Vec<crate::modules::library::application::mods::organizer_move::OrganizerMovePathHint>,
    lease: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    let config = app
        .try_state::<crate::modules::settings::application::config::ConfigService>()
        .ok_or_else(|| {
            AppError::Internal("ConfigService state missing for disk reconcile".to_string())
        })?;
    let watcher = app
        .try_state::<crate::modules::workspace::application::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState missing for disk reconcile".to_string()))?;
    let state = app.try_state::<DiskReconcileState>().ok_or_else(|| {
        AppError::Internal("DiskReconcileState missing for disk reconcile".to_string())
    })?;
    let operation_lock = app
        .try_state::<crate::platform::fs::operation_lock::OperationLock>()
        .ok_or_else(|| {
            AppError::Internal("OperationLock missing for disk reconcile".to_string())
        })?;
    let reason = DiskReconcileReason::InternalMutation;
    let request = DiskReconcileRequest::manual_with_path_hints(
        game_id.to_string(),
        reason.clone(),
        changed_paths,
        path_hints
            .into_iter()
            .map(
                |hint| crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcilePathHint {
                    old_path: hint.old_path,
                    new_path: hint.new_path,
                    target_object_id: hint.target_object_id,
                },
            )
            .collect(),
    );

    crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_under_lease(
        DiskReconcileContext {
            pool,
            config: config.inner(),
            state: state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: operation_lock.inner(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id,
                    reason,
                ),
            )),
        },
        request,
        lease,
    )
    .await
}

async fn run_initial_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    run_internal_disk_reconcile_with_options(
        app,
        pool,
        game_id,
        Vec::new(),
        DiskReconcileReason::StartupBoot,
        true,
        false,
        Vec::new(),
    )
    .await
}

/// Single-flight activation/startup recovery. The first caller performs the
/// full disk scan; concurrent and later readers receive the same terminal
/// result, including an explicit failure instead of silently reading stale DB
/// state.
pub async fn ensure_initial_disk_recovery(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    state: &DiskReconcileState,
    game_id: &str,
) -> InitialRecoveryOutcome {
    match state.claim_initial_recovery(game_id).await {
        InitialRecoveryClaim::Run { generation } => {
            let outcome = match run_initial_disk_reconcile(app, pool, game_id).await {
                Ok(result) => InitialRecoveryOutcome::Completed(Box::new(result)),
                Err(error) => InitialRecoveryOutcome::Failed(error.to_string()),
            };
            state.finish_initial_recovery(game_id, generation, outcome.clone());
            outcome
        }
        InitialRecoveryClaim::Finished(outcome) => outcome,
    }
}

/// Starts first recovery in the background and immediately returns a read
/// readiness state. The workspace may render a last-valid projection while
/// this runs; mutation preflight remains responsible for waiting/rejecting.
pub fn start_initial_disk_recovery(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    state: &DiskReconcileState,
    game_id: &str,
) -> crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::{
        InitialRecoveryReadiness, InitialRecoveryStart,
    };

    match state.start_initial_recovery(game_id) {
        InitialRecoveryStart::Run { generation } => {
            let app = app.clone();
            let pool = pool.clone();
            let game_id = game_id.to_string();
            tauri::async_runtime::spawn(async move {
                let outcome = match run_initial_disk_reconcile(&app, &pool, &game_id).await {
                    Ok(result) => InitialRecoveryOutcome::Completed(Box::new(result)),
                    Err(error) => InitialRecoveryOutcome::Failed(error.to_string()),
                };
                let state = app.state::<DiskReconcileState>();
                state.finish_initial_recovery(&game_id, generation, outcome.clone());
                let result = match outcome {
                    InitialRecoveryOutcome::Completed(result) => *result,
                    InitialRecoveryOutcome::Failed(error) => {
                        log::warn!("Initial disk recovery failed for '{game_id}': {error}");
                        return;
                    }
                };
                if let Err(error) = app.emit("disk_reconcile:result", result) {
                    log::warn!("Could not emit initial disk recovery result: {error}");
                }
            });
            InitialRecoveryReadiness::Syncing { generation }
        }
        InitialRecoveryStart::Syncing { generation } => {
            InitialRecoveryReadiness::Syncing { generation }
        }
        InitialRecoveryStart::Finished(_) => state.initial_recovery_readiness(game_id),
    }
}

#[allow(clippy::too_many_arguments)] // Keeps one explicit request boundary without another overlapping options model.
async fn run_internal_disk_reconcile_with_options(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
    reason: DiskReconcileReason,
    force_full: bool,
    emit_when_blocked: bool,
    path_hints: Vec<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcilePathHint>,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    let config = app
        .try_state::<crate::modules::settings::application::config::ConfigService>()
        .ok_or_else(|| {
            AppError::Internal("ConfigService state missing for disk reconcile".to_string())
        })?;
    let watcher = app
        .try_state::<crate::modules::workspace::application::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState missing for disk reconcile".to_string()))?;
    let disk_reconcile_state = app.try_state::<DiskReconcileState>().ok_or_else(|| {
        AppError::Internal("DiskReconcileState missing for disk reconcile".to_string())
    })?;
    let operation_lock = app
        .try_state::<crate::platform::fs::operation_lock::OperationLock>()
        .ok_or_else(|| {
            AppError::Internal("OperationLock missing for disk reconcile".to_string())
        })?;

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool,
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
            operation_lock: operation_lock.inner(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id,
                    reason.clone(),
                ),
            )),
        },
        if path_hints.is_empty() {
            DiskReconcileRequest::manual(game_id.to_string(), reason, changed_paths, force_full)
        } else {
            DiskReconcileRequest::manual_with_path_hints(
                game_id.to_string(),
                reason,
                changed_paths,
                path_hints,
            )
        },
    )
    .await?;
    if emit_when_blocked
        && (requires_user_resolution(&result.status)
            || result.status == DiskReconcileStatus::AppliedWithFolderConflicts)
    {
        app.emit("disk_reconcile:result", &result)?;
    }
    Ok(result)
}

pub async fn run_internal_disk_reconcile_with_path_hints(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
    path_hints: Vec<crate::modules::library::application::mods::organizer_move::OrganizerMovePathHint>,
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
    run_internal_disk_reconcile_with_options(
        app,
        pool,
        game_id,
        changed_paths,
        DiskReconcileReason::InternalMutation,
        false,
        true,
        path_hints
            .into_iter()
            .map(
                |hint| crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcilePathHint {
                    old_path: hint.old_path,
                    new_path: hint.new_path,
                    target_object_id: hint.target_object_id,
                },
            )
            .collect(),
    )
    .await
}

pub async fn emit_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
) -> Result<(), AppError> {
    let result = run_internal_disk_reconcile(app, pool, game_id, changed_paths).await?;
    if !requires_user_resolution(&result.status) {
        app.emit("disk_reconcile:result", result)?;
    }
    Ok(())
}

pub async fn ensure_mutation_preflight(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<(), AppError> {
    ensure_mutation_preflight_for_paths(app, pool, game_id, None).await
}

/// Opening a folder never mutates disk or the projection. Validate the target
/// remains inside the configured Mods root (at the command boundary) and use
/// only the ephemeral identity census to reject an ambiguous candidate.
pub async fn ensure_open_path_preflight(
    app: &tauri::AppHandle,
    game_id: &str,
    path: &std::path::Path,
) -> Result<(), AppError> {
    if let Some(state) = app.try_state::<DiskReconcileState>() {
        if !initial_recovery_allows_mutation(state.initial_recovery_readiness(game_id)) {
            return Err(AppError::Io(
                "Mods are still synchronizing with disk. Try again when sync completes."
                    .to_string(),
            ));
        }
    }
    let config = app
        .try_state::<crate::modules::settings::application::config::ConfigService>()
        .ok_or_else(|| {
            AppError::Internal("ConfigService state missing for disk reconcile".to_string())
        })?;
    let mods_path = config
        .get_settings()
        .games
        .into_iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}' not found")))?
        .mod_path;
    let census_path = mods_path.clone();
    let census = tokio::task::spawn_blocking(move || {
        crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::collect_disk_identity_census(&census_path)
    })
    .await?
    .map_err(|error| AppError::Io(error.into_message()))?;
    let conflicts = crate::modules::reconciliation::application::disk_reconcile::identity_conflicts::detect_folder_name_conflicts_from_census(
        game_id,
        &census,
    );
    let paths = [path.to_string_lossy().to_string()];
    if conflicts_intersect_paths(&conflicts, &paths) {
        return Err(AppError::Io(
            "Resolve folder name conflicts affecting this mod before opening it".to_string(),
        ));
    }
    Ok(())
}

pub async fn ensure_mutation_preflight_for_paths(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    paths: Option<&[String]>,
) -> Result<(), AppError> {
    if let Some(state) = app.try_state::<DiskReconcileState>() {
        if !initial_recovery_allows_mutation(state.initial_recovery_readiness(game_id)) {
            return Err(AppError::Io(
                "Mods are still synchronizing with disk. Try again when sync completes."
                    .to_string(),
            ));
        }
    }
    let result = match paths {
        Some(paths) if !paths.is_empty() => {
            run_internal_disk_reconcile(app, pool, game_id, paths.to_vec()).await?
        }
        Some(_) | None => run_full_internal_disk_reconcile(app, pool, game_id).await?,
    };
    if result.status == DiskReconcileStatus::AppliedWithFolderConflicts {
        app.emit("disk_reconcile:result", &result)?;
        if paths.is_none_or(|paths| conflicts_intersect_paths(&result.folder_conflicts, paths)) {
            return Err(AppError::Io(
                "Resolve folder name conflicts affecting this mod before modifying it".to_string(),
            ));
        }
    }
    if result.status == DiskReconcileStatus::NeedsRenameConfirmation {
        app.emit("disk_reconcile:result", &result)?;
        return Err(AppError::Io(
            "Review external folder rename candidates before modifying the mods directory"
                .to_string(),
        ));
    }
    if result.status == DiskReconcileStatus::SourceUnavailable {
        return Err(AppError::Io(
            result
                .error_message
                .unwrap_or_else(|| "Mods folder is unavailable".to_string()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod committed_mutation_tests {
    use super::*;
    use crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind;

    #[test]
    fn committed_reconcile_failure_becomes_retryable_warning_instead_of_command_error() {
        let settlement = settle_committed_reconcile(Err(AppError::Io(
            "injected projection failure".to_string(),
        )));

        assert!(settlement.reconcile.is_none());
        let warning = settlement
            .sync_warning
            .expect("committed mutation must retain a non-fatal sync warning");
        assert_eq!(
            warning.kind,
            CommittedMutationSyncWarningKind::ReconcileFailed
        );
        assert!(warning.message.contains("injected projection failure"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    #[test]
    fn pending_initial_recovery_rejects_disk_mutations() {
        assert!(!initial_recovery_allows_mutation(
            InitialRecoveryReadiness::Syncing { generation: 3 }
        ));
    }

    #[test]
    fn conflict_scope_blocks_only_its_candidate_and_descendants() {
        let scope = "E:/Mods/Alice";
        assert!(path_is_within_conflict_scope("E:/Mods/Alice/Blue", scope));
        assert!(path_is_within_conflict_scope("e:\\mods\\alice", scope));
        assert!(!path_is_within_conflict_scope("E:/Mods/Bob/Blue", scope));
    }

    #[test]
    fn conflict_scope_blocks_parent_mutations_that_cover_a_candidate() {
        assert!(path_is_within_conflict_scope(
            "E:/Mods/Alice",
            "E:/Mods/Alice/Variants/Blue"
        ));
    }

    #[test]
    fn conflict_scope_treats_windows_verbatim_and_regular_paths_as_equal() {
        assert!(path_is_within_conflict_scope(
            r"\\?\C:\Mods\Alice\Blue",
            r"C:\Mods\Alice\Blue"
        ));
    }

    #[test]
    fn path_scoped_mutation_preflight_does_not_use_full_reconcile() {
        let source = include_str!("emit.rs");
        let start = source
            .find("pub async fn ensure_mutation_preflight_for_paths(")
            .expect("preflight source");
        let end = source
            .find("#[cfg(test)]")
            .expect("preflight test boundary");
        let preflight = &source[start..end];

        assert!(
            preflight.contains("run_internal_disk_reconcile(app, pool, game_id, paths.to_vec())"),
            "path-scoped preflight must classify its affected roots only"
        );
    }
}
