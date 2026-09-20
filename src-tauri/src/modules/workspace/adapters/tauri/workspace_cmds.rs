use std::time::Instant;
use tauri::State;

use crate::modules::library::api::{
    bulk_delete_mods_from_snapshot, bulk_pin_mods_from_snapshot, bulk_set_mod_safety_from_snapshot,
    bulk_toggle_favorite_from_snapshot, bulk_toggle_mods_from_snapshot,
    bulk_update_info_from_snapshot, move_mods_to_object_from_snapshot, BulkCancelState,
    MoveModsToObjectInput,
};
use crate::modules::mutation::api::StepSettlement;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::emit::require_applied_reconcile;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::modules::workspace::application::workspace::switch::PreparedWorkspaceExecutionError;
use crate::modules::workspace::domain::workspace::{
    WorkspaceExplorerBulkAction, WorkspaceExplorerBulkInput, WorkspaceExplorerPage,
    WorkspaceExplorerPageInput, WorkspacePreviewInput, WorkspacePreviewResult,
    WorkspaceStructureInput, WorkspaceStructureViewModel, WorkspaceSwitchInput,
    WorkspaceSwitchResult,
};
use crate::shared::errors::AppError;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_structure(
    app: tauri::AppHandle,
    input: WorkspaceStructureInput,
    pool: State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<WorkspaceStructureViewModel, AppError> {
    let game_id = input.filter.game_id.clone();
    let recovery_readiness = crate::modules::reconciliation::application::disk_reconcile::emit::start_initial_disk_recovery(
        &app,
        pool.inner(),
        disk_reconcile_state.inner(),
        &game_id,
    );

    let mut workspace = crate::modules::workspace::application::workspace::get_workspace_structure_with_listing_mode(
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

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_explorer_page(
    input: WorkspaceExplorerPageInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<WorkspaceExplorerPage, AppError> {
    let mods_path = crate::modules::workspace::application::workspace::load_game_mods_path(
        pool.inner(),
        &input.query.game_id,
    )
    .await?;
    crate::modules::workspace::application::explorer::listing::list_workspace_explorer_page(
        pool.inner(),
        mods_path,
        input,
    )
    .await
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn execute_workspace_explorer_bulk(
    app: tauri::AppHandle,
    input: WorkspaceExplorerBulkInput,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
) -> Result<crate::modules::library::application::mods::bulk::BulkResult, AppError> {
    let game_id = input.selection.query.game_id.clone();
    let mods_path = crate::modules::workspace::application::workspace::load_game_mods_path(
        pool.inner(),
        &game_id,
    )
    .await?;
    let resolved = crate::modules::workspace::application::explorer::listing::resolve_workspace_explorer_selection(
        mods_path,
        input.selection,
    )
    .await?;
    crate::modules::workspace::application::explorer::listing::validate_workspace_explorer_selection_identities(
        &resolved.expected_identities,
    )?;
    let paths = resolved.paths;
    let expected_identities = resolved.expected_identities;

    match input.action {
        WorkspaceExplorerBulkAction::Toggle {
            enable,
            operation_id,
        } => {
            bulk_toggle_mods_from_snapshot(
                app,
                config,
                pool,
                watcher,
                disk_reconcile,
                op_lock,
                game_id,
                paths,
                enable,
                operation_id,
                expected_identities,
            )
            .await
        }
        WorkspaceExplorerBulkAction::Delete { operation_id } => {
            bulk_delete_mods_from_snapshot(
                app,
                config,
                pool,
                watcher,
                disk_reconcile,
                op_lock,
                cancel_state,
                game_id,
                paths,
                operation_id,
                expected_identities,
            )
            .await
        }
        WorkspaceExplorerBulkAction::UpdateInfo { update } => {
            bulk_update_info_from_snapshot(
                app,
                config,
                pool,
                watcher,
                disk_reconcile,
                op_lock,
                game_id,
                paths,
                update,
                expected_identities,
            )
            .await
        }
        WorkspaceExplorerBulkAction::SetSafety { safe } => {
            bulk_set_mod_safety_from_snapshot(
                app,
                config,
                pool,
                watcher,
                disk_reconcile,
                op_lock,
                game_id,
                paths,
                safe,
                expected_identities,
            )
            .await
        }
        WorkspaceExplorerBulkAction::SetFavorite { favorite } => {
            bulk_toggle_favorite_from_snapshot(
                app,
                config,
                pool,
                watcher,
                disk_reconcile,
                op_lock,
                game_id,
                paths,
                favorite,
                expected_identities,
            )
            .await
        }
        WorkspaceExplorerBulkAction::SetPin { pin } => {
            bulk_pin_mods_from_snapshot(
                app,
                config,
                pool,
                watcher,
                disk_reconcile,
                op_lock,
                game_id,
                paths,
                pin,
                expected_identities,
            )
            .await
        }
        WorkspaceExplorerBulkAction::MoveToObject {
            target_object_id,
            target_subpath,
            status,
        } => {
            move_mods_to_object_from_snapshot(
                app,
                config,
                pool,
                op_lock,
                disk_reconcile,
                watcher,
                MoveModsToObjectInput {
                    game_id,
                    folder_paths: paths,
                    target_object_id,
                    target_subpath,
                    status,
                },
                expected_identities,
            )
            .await
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_preview(
    input: WorkspacePreviewInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<WorkspacePreviewResult, AppError> {
    crate::modules::workspace::application::workspace::get_workspace_preview(pool.inner(), input)
        .await
}

fn workspace_recovery_status(
    readiness: crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness,
) -> crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
    use crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus;

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
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchResult, AppError> {
    execute_workspace_switch_request(
        app,
        WorkspaceSwitchRequest::Single(input),
        config,
        pool,
        watcher_state,
        disk_reconcile_state,
        op_lock,
    )
    .await
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn execute_workspace_object_bulk_switch(
    app: tauri::AppHandle,
    game_id: String,
    object_ids: Vec<String>,
    desired_enabled: bool,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchResult, AppError> {
    execute_workspace_switch_request(
        app,
        WorkspaceSwitchRequest::ObjectBatch {
            game_id,
            object_ids,
            desired_enabled,
        },
        config,
        pool,
        watcher_state,
        disk_reconcile_state,
        op_lock,
    )
    .await
}

enum WorkspaceSwitchRequest {
    Single(WorkspaceSwitchInput),
    ObjectBatch {
        game_id: String,
        object_ids: Vec<String>,
        desired_enabled: bool,
    },
}

impl WorkspaceSwitchRequest {
    fn game_id(&self) -> &str {
        match self {
            Self::Single(input) => &input.game_id,
            Self::ObjectBatch { game_id, .. } => game_id,
        }
    }

    async fn prepare(
        &self,
        config: &ConfigService,
        pool: &sqlx::SqlitePool,
    ) -> Result<
        crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch,
        AppError,
    > {
        match self {
            Self::Single(input) => {
                crate::modules::workspace::application::workspace::switch::prepare_switch(
                    input, config, pool,
                )
                .await
            }
            Self::ObjectBatch {
                game_id,
                object_ids,
                desired_enabled,
            } => {
                crate::modules::workspace::application::workspace::switch::prepare_object_batch_switch(
                    pool,
                    game_id,
                    object_ids,
                    *desired_enabled,
                )
                .await
            }
        }
    }
}

async fn execute_workspace_switch_request(
    app: tauri::AppHandle,
    request: WorkspaceSwitchRequest,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchResult, AppError> {
    let started_at = Instant::now();
    let game_id = request.game_id().to_string();
    let _foreground_intent = op_lock.inner_lock().foreground_intent();
    let lock_wait_started_at = Instant::now();
    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    let lock_wait_elapsed = lock_wait_started_at.elapsed();
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_recovery_allows_mutation(
        &app,
        &game_id,
    )?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let prepare_started_at = Instant::now();
    let mut prepared = request.prepare(config.inner(), pool.inner()).await?;
    let prepare_elapsed = prepare_started_at.elapsed();
    if let Some(result) = prepared.immediate_result() {
        log::debug!(
            "workspace switch timing outcome=immediate lock_wait_ms={} prepare_ms={} total_ms={}",
            lock_wait_elapsed.as_millis(),
            prepare_elapsed.as_millis(),
            started_at.elapsed().as_millis(),
        );
        return Ok(result);
    }
    let mut scope = prepared.mutation_scope(&mods_root)?;
    if scope.renames.is_empty() {
        let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
            )
            .await?;
        let execute_started_at = Instant::now();
        let result = prepared.execute(&app, &watcher_state);
        log::debug!(
            "workspace switch timing outcome=exempt lock_wait_ms={} prepare_ms={} execute_ms={} total_ms={}",
            lock_wait_elapsed.as_millis(),
            prepare_elapsed.as_millis(),
            execute_started_at.elapsed().as_millis(),
            started_at.elapsed().as_millis(),
        );
        return result;
    }
    scope.validate_identities()?;
    let trusted_preflight_scope = trusted_toggle_scope(&scope, &mods_root)
        && disk_reconcile_state.trusted_regional_mutation_allowed(&game_id, &mods_root);
    let preflight_started_at = Instant::now();
    let preflight_guard = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::Reconciliation)
        .await?;
    let mut preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_under_game_lock(
        &app,
        pool.inner(),
        &game_id,
        scope.changed_paths.clone(),
        trusted_preflight_scope,
        &game_guard,
        preflight_guard.op_guard(),
    )
    .await?;
    drop(preflight_guard);

    prepared = request.prepare(config.inner(), pool.inner()).await?;
    if let Some(result) = prepared.immediate_result() {
        return Ok(result);
    }
    let refreshed_scope = prepared.mutation_scope(&mods_root)?;
    if !same_mutation_paths(&scope.changed_paths, &refreshed_scope.changed_paths) {
        refreshed_scope.validate_identities()?;
        let trusted_refreshed_scope = trusted_toggle_scope(&refreshed_scope, &mods_root)
            && disk_reconcile_state.trusted_regional_mutation_allowed(&game_id, &mods_root);
        let second_preflight_guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::Reconciliation,
            )
            .await?;
        preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_under_game_lock(
            &app,
            pool.inner(),
            &game_id,
            refreshed_scope.changed_paths.clone(),
            trusted_refreshed_scope,
            &game_guard,
            second_preflight_guard.op_guard(),
        )
        .await?;
        drop(second_preflight_guard);
        prepared = request.prepare(config.inner(), pool.inner()).await?;
        if let Some(result) = prepared.immediate_result() {
            return Ok(result);
        }
        let final_scope = prepared.mutation_scope(&mods_root)?;
        if !same_mutation_paths(&refreshed_scope.changed_paths, &final_scope.changed_paths) {
            return Err(AppError::Io(
                "Workspace changed repeatedly during regional preflight; retry the switch"
                    .to_string(),
            ));
        }
        scope = final_scope;
    } else {
        scope = refreshed_scope;
    }
    let preflight_elapsed = preflight_started_at.elapsed();
    if scope.renames.is_empty() {
        let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
            )
            .await?;
        return prepared.execute(&app, &watcher_state);
    }
    if crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
        &preflight.folder_conflicts,
        &scope.changed_paths,
    ) {
        return Err(
            crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error(),
        );
    }
    scope.validate_identities()?;
    let trusted_scope_candidate = trusted_toggle_scope(&scope, &mods_root)
        && disk_reconcile_state.trusted_regional_mutation_allowed(&game_id, &mods_root);
    let journal_steps = scope
        .renames
        .iter()
        .map(|rename| {
            crate::modules::mutation::api::PlannedStep::rename(
                rename.sequence,
                rename.old_path.clone(),
                rename.new_path.clone(),
            )
            .with_expected_identity(rename.expected_identity.clone())
        })
        .collect::<Vec<_>>();
    let journal_started_at = Instant::now();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "workspace-switch",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let journal_prepare_elapsed = journal_started_at.elapsed();
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    if let Err(error) = scope.validate_identities() {
        mutation_lease.begin_rollback()?;
        let settlements = scope
            .renames
            .iter()
            .map(|rename| (rename.sequence, StepSettlement::RolledBack))
            .collect::<Vec<_>>();
        mutation_lease.settle_steps(&settlements)?;
        mutation_lease.finish_rollback()?;
        return Err(error);
    }
    let trusted_mutation = trusted_scope_candidate
        .then(|| watcher_state.current_session_for_root(&mods_root))
        .flatten()
        .and_then(|session| {
            disk_reconcile_state
                .trusted_internal_mutation_evidence(
                    &game_id,
                    &mods_root,
                    session.generation(),
                )
                .map(|authority| (authority, session))
        })
        .and_then(|(authority, session)| {
            scope
                .renames
                .iter()
                .map(|rename| {
                    rename.expected_identity.clone().map(|expected_identity| {
                        crate::modules::workspace::application::scanner::watcher::ExpectedRenameEcho {
                            old_path: rename.old_path.clone(),
                            new_path: rename.new_path.clone(),
                            expected_identity,
                        }
                    })
                })
                .collect::<Option<Vec<_>>>()
                .map(|renames| (authority, session, renames))
        });
    let expected_echo_evidence = trusted_mutation.as_ref().map(|(_, session, renames)| {
        watcher_state
            .suppressor
            .expect_rename_echoes(&game_id, session, renames.clone())
    });
    let trusted_scope = trusted_mutation.is_some();
    let execute_started_at = Instant::now();
    let mut result = match prepared.execute_with_outcome(&app, &watcher_state) {
        Ok(result) => result,
        Err(PreparedWorkspaceExecutionError::Apply(error)) => {
            if let Some(evidence) = &expected_echo_evidence {
                watcher_state
                    .suppressor
                    .discard_expected_rename_echoes(evidence);
            }
            disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
            mutation_lease.begin_rollback()?;
            let settlements = scope
                .renames
                .iter()
                .map(|rename| (rename.sequence, StepSettlement::RolledBack))
                .collect::<Vec<_>>();
            mutation_lease.settle_steps(&settlements)?;
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
        Err(PreparedWorkspaceExecutionError::Compensation(error)) => {
            if let Some(evidence) = &expected_echo_evidence {
                watcher_state
                    .suppressor
                    .discard_expected_rename_echoes(evidence);
            }
            disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
            mutation_lease.fail(format!(
                "Workspace compensation failed; recovery is required: {error}"
            ))?;
            return Err(error);
        }
    };
    let execute_elapsed = execute_started_at.elapsed();
    let applied_settlements = scope
        .renames
        .iter()
        .map(|rename| (rename.sequence, StepSettlement::Applied))
        .collect::<Vec<_>>();
    if let Err(error) = mutation_lease.settle_steps(&applied_settlements) {
        if let Some(evidence) = &expected_echo_evidence {
            watcher_state
                .suppressor
                .discard_expected_rename_echoes(evidence);
        }
        disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
        return Err(error);
    }
    let reconcile_started_at = Instant::now();
    let reconcile = if trusted_scope {
        crate::modules::reconciliation::application::disk_reconcile::emit::run_trusted_internal_disk_reconcile_under_lease(
            &app,
            pool.inner(),
            &game_id,
            scope.changed_paths.clone(),
            &mutation_lease,
        )
        .await
    } else {
        crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(
            &app,
            pool.inner(),
            &game_id,
            &mutation_lease,
        )
        .await
    }
    .and_then(require_applied_reconcile);
    match reconcile {
        Ok(reconcile) => {
            let journal_commit_started_at = Instant::now();
            if let Err(error) = mutation_lease.mark_db_committed() {
                if let Some(evidence) = &expected_echo_evidence {
                    watcher_state
                        .suppressor
                        .discard_expected_rename_echoes(evidence);
                }
                disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
                return Err(error);
            }
            if let Err(error) = mutation_lease.commit() {
                if let Some(evidence) = &expected_echo_evidence {
                    watcher_state
                        .suppressor
                        .discard_expected_rename_echoes(evidence);
                }
                disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
                return Err(error);
            }
            if let Some((authority, _, _)) = &trusted_mutation {
                if !disk_reconcile_state.mark_trusted_internal_mutation_reconciled(
                    authority,
                    &reconcile,
                    &scope.changed_paths,
                ) {
                    disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
                }
            }
            let journal_commit_elapsed = journal_commit_started_at.elapsed();
            let runtime_queue_started_at = Instant::now();
            let runtime_changed_paths = scope
                .renames
                .iter()
                .map(|rename| rename.new_path.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let runtime_request =
                crate::modules::reconciliation::api::runtime_sync_request_for_changed_paths(
                    pool.inner(),
                    &game_id,
                    &mods_root,
                    &runtime_changed_paths,
                    &scope.owning_roots,
                )
                .await;
            let generation = crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
                &app,
                pool.inner(),
                &game_id,
                crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
                runtime_request,
            );
            result.runtime_sync_generation = Some(generation);
            log::debug!(
                "workspace switch timing outcome=applied scan_scope={:?} full_scan_count={} roots={} preflight_ms={} lock_wait_ms={} prepare_ms={} journal_prepare_ms={} rename_ms={} reconcile_ms={} journal_commit_ms={} runtime_queue_ms={} runtime_wait_ms=0 total_ms={}",
                reconcile.scan_scope,
                usize::from(reconcile.scan_scope == crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::Full),
                scope.owning_roots.len(),
                preflight_elapsed.as_millis(),
                lock_wait_elapsed.as_millis(),
                prepare_elapsed.as_millis(),
                journal_prepare_elapsed.as_millis(),
                execute_elapsed.as_millis(),
                reconcile_started_at.elapsed().as_millis(),
                journal_commit_elapsed.as_millis(),
                runtime_queue_started_at.elapsed().as_millis(),
                started_at.elapsed().as_millis(),
            );
            Ok(result)
        }
        Err(error) => {
            if let Some(evidence) = &expected_echo_evidence {
                watcher_state
                    .suppressor
                    .discard_expected_rename_echoes(evidence);
            }
            disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) = prepared.rollback(&watcher_state) {
                let combined = format!("{error}; workspace rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            let rolled_back_settlements = scope
                .renames
                .iter()
                .map(|rename| (rename.sequence, StepSettlement::RolledBack))
                .collect::<Vec<_>>();
            mutation_lease.settle_steps(&rolled_back_settlements)?;
            let rollback_projection = if trusted_scope {
                match crate::modules::reconciliation::application::disk_reconcile::emit::run_trusted_internal_disk_reconcile_under_lease(
                    &app,
                    pool.inner(),
                    &game_id,
                    scope.changed_paths.clone(),
                    &mutation_lease,
                )
                .await
                .and_then(require_applied_reconcile)
                {
                    Ok(result) => Ok(result),
                    Err(scoped_error) => {
                        log::warn!("Scoped rollback projection failed; escalating to full scan: {scoped_error}");
                        crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                            &app,
                            pool.inner(),
                            &game_id,
                            &mutation_lease,
                        )
                        .await
                        .and_then(require_applied_reconcile)
                    }
                }
            } else {
                crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                    &app,
                    pool.inner(),
                    &game_id,
                    &mutation_lease,
                )
                .await
                .and_then(require_applied_reconcile)
            };
            if let Err(rollback_reconcile_error) = rollback_projection {
                let combined =
                    format!("{error}; rollback projection failed: {rollback_reconcile_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            mutation_lease.finish_rollback()?;
            Err(error)
        }
    }
}

fn same_mutation_paths(left: &[String], right: &[String]) -> bool {
    let normalized = |paths: &[String]| {
        paths
            .iter()
            .map(|path| {
                crate::shared::path_key::canonical_path_key_for_path(std::path::Path::new(path))
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    normalized(left) == normalized(right)
}

fn trusted_toggle_scope(
    scope: &crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope,
    mods_root: &std::path::Path,
) -> bool {
    if !scope.has_trusted_identities() {
        return false;
    }
    scope.renames.iter().all(|rename| {
        rename.old_path.starts_with(mods_root)
            && rename.new_path.starts_with(mods_root)
            && rename.old_path.parent() == rename.new_path.parent()
            && crate::shared::path_key::folder_path_key(
                &rename
                    .old_path
                    .strip_prefix(mods_root)
                    .unwrap_or(&rename.old_path)
                    .to_string_lossy(),
                None,
            ) == crate::shared::path_key::folder_path_key(
                &rename
                    .new_path
                    .strip_prefix(mods_root)
                    .unwrap_or(&rename.new_path)
                    .to_string_lossy(),
                None,
            )
    })
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

    #[test]
    fn trusted_scope_accepts_identity_preserving_toggle_batches() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        let second_old_path = temp.path().join("DISABLED Red");
        let second_new_path = temp.path().join("Red");
        std::fs::create_dir(&old_path).expect("source folder");
        std::fs::create_dir(&second_old_path).expect("second source folder");
        let scope = crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope {
            renames: vec![
                crate::modules::workspace::application::workspace::switch::WorkspaceMutationRename {
                    sequence: 0,
                    old_path: old_path.clone(),
                    new_path: new_path.clone(),
                    identity_path: old_path.clone(),
                    expected_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_path),
                },
                crate::modules::workspace::application::workspace::switch::WorkspaceMutationRename {
                    sequence: 1,
                    old_path: second_old_path.clone(),
                    new_path: second_new_path.clone(),
                    identity_path: second_old_path.clone(),
                    expected_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&second_old_path),
                },
            ],
            changed_paths: vec![
                old_path.to_string_lossy().into_owned(),
                new_path.to_string_lossy().into_owned(),
                second_old_path.to_string_lossy().into_owned(),
                second_new_path.to_string_lossy().into_owned(),
            ],
            owning_roots: vec![
                "DISABLED Blue".to_string(),
                "Blue".to_string(),
                "DISABLED Red".to_string(),
                "Red".to_string(),
            ],
            touched_object_ids: Vec::new(),
        };
        assert!(trusted_toggle_scope(&scope, temp.path()));
    }
}
