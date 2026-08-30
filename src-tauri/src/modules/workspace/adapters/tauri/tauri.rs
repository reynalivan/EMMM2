
// --- From commands/app/workspace_cmds.rs ---
use tauri::State;

use crate::shared::errors::AppError;
use crate::modules::workspace::domain::workspace::{
    WorkspaceSwitchInput, WorkspaceSwitchResult, WorkspaceViewModel, WorkspaceViewModelInput,
};
use crate::modules::settings::application::config::ConfigService;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::workspace::application::scanner::watcher::WatcherState;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_view_model(
    app: tauri::AppHandle,
    input: WorkspaceViewModelInput,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<WorkspaceViewModel, AppError> {
    let game_id = input.filter.game_id.clone();
    let recovery_readiness = crate::modules::reconciliation::application::disk_reconcile::emit::start_initial_disk_recovery(
        &app,
        pool.inner(),
        disk_reconcile_state.inner(),
        &game_id,
    );

    let mut workspace = crate::modules::workspace::application::workspace::get_workspace_view_model_with_listing_mode(
        pool.inner(),
        input,
        matches!(
            recovery_readiness,
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Syncing { .. }
                | crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness::Unstarted { .. }
        ),
    )
    .await?;
    workspace.runtime.recovery_status = workspace_recovery_status(recovery_readiness);
    Ok(workspace)
}

fn workspace_recovery_status(
    readiness: crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness,
) -> crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus {
    use crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus;
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    match readiness {
        InitialRecoveryReadiness::Ready { .. } => WorkspaceRecoveryStatus::Ready,
        InitialRecoveryReadiness::Failed { .. } => WorkspaceRecoveryStatus::Failed,
        InitialRecoveryReadiness::Unstarted { .. } | InitialRecoveryReadiness::Syncing { .. } => {
            WorkspaceRecoveryStatus::Syncing
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn execute_workspace_switch(
    app: tauri::AppHandle,
    input: WorkspaceSwitchInput,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchResult, AppError> {
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &input.game_id,
    )
    .await?;
    let op_guard = op_lock.acquire().await?;
    let game_id = input.game_id.clone();
    let result = crate::modules::workspace::application::workspace::switch::execute_switch(
        input,
        config.inner(),
        pool.inner(),
        watcher_state.inner(),
        op_guard.op_guard(),
    )
    .await;
    drop(op_guard);
    let mut result = match result {
        Ok(result) => result,
        Err(error) => {
            let reconcile =
                crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
                    &app,
                    pool.inner(),
                    &game_id,
                )
                .await;
            return match reconcile {
                Ok(_) => Err(error),
                Err(reconcile_error) => Err(AppError::Io(format!(
                    "{error}; convergence also failed: {reconcile_error}"
                ))),
            };
        }
    };
    if !result.changed_folder_paths.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.changed_folder_paths.clone(),
            )
            .await,
        );
        result.sync_warning = settlement.sync_warning;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    #[test]
    fn pending_recovery_maps_to_syncing_workspace_runtime() {
        assert_eq!(
            workspace_recovery_status(InitialRecoveryReadiness::Syncing { generation: 7 }),
            crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus::Syncing
        );
    }
}


// --- From commands/scanner/disk_reconcile_cmds.rs ---
use crate::shared::errors::AppError;
use std::path::{Component, Path};
use tauri::State;

use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason;

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
    let _activation_guard = disk_reconcile_state.activation_guard().await;
    let game_lock = disk_reconcile_state.game_lock(&request.game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = operation_lock.acquire().await?;
    let result = crate::modules::reconciliation::application::disk_reconcile::source_recovery::apply_game_mods_directory(
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
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
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
            app,
            game_id.clone(),
            reason.clone(),
        ),
    );
    crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state(
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
        ),
    )
    .await
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
) -> Result<crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult, AppError> {
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


// --- From commands/scanner/folder_entries_cmds.rs ---
//! Read-only folder inspection commands used by workspace tooltips.

use crate::shared::errors::AppError;

#[tauri::command]
#[specta::specta]
pub async fn list_folder_entries_cmd(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_path: String,
    game_id: String,
) -> Result<Vec<crate::modules::workspace::application::scanner::folder_entries::FolderEntry>, AppError> {
    Ok(
        crate::modules::workspace::application::scanner::folder_entries::list_folder_entries(
            pool.inner(),
            &game_id,
            &folder_path,
        )
        .await?,
    )
}


// --- From commands/scanner/watcher_cmds.rs ---
//! File system watcher commands.

use crate::shared::sync::lock;
use crate::shared::errors::AppError;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use tauri::State;

/// Start the file watcher for a specific path.
/// Emits `mod_watch:event` to the frontend.
///
/// Delegates the full lifecycle (thread spawning, event loop, DB sync) to
/// `services::scanner::watcher::lifecycle::start_watcher`.
#[tauri::command]
#[specta::specta]
pub async fn start_watcher(
    app: tauri::AppHandle,
    path: String,
    game_id: String,
    state: State<'_, WatcherState>,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<(), AppError> {
    let configured_root =
        crate::platform::fs::guard::validate_mods_root(&config, &game_id, &path)?;
    let db_pool = (*pool).clone();
    Ok(crate::modules::workspace::application::scanner::watcher::lifecycle::start_watcher(
        app,
        &state,
        db_pool,
        configured_root.to_string_lossy().into_owned(),
        game_id,
    )?)
}

/// Stop the file watcher. Cleanly drops the `RecommendedWatcher`,
/// terminating the background event loop thread.
///
/// Called by the frontend in `useEffect` cleanup when the active game changes
/// or the component unmounts.
///
/// # Covers: req-05 AC-05.2.2, req-28 (Game Switch → stop → init)
#[tauri::command]
#[specta::specta]
pub async fn stop_watcher(watcher: State<'_, WatcherState>) -> Result<(), AppError> {
    watcher.invalidate_session();
    let mut w = lock(&watcher.watcher);
    if w.is_some() {
        log::info!("Stopping watcher via command");
        *w = None;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/watcher_cmds_tests.rs"]
mod tests;

