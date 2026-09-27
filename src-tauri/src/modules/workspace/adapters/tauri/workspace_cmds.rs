use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

use crate::modules::library::api::{
    bulk_delete_mods_from_snapshot, bulk_pin_mods_from_snapshot, bulk_set_mod_safety_from_snapshot,
    bulk_toggle_favorite_from_snapshot, bulk_toggle_mods_from_snapshot,
    bulk_update_info_from_snapshot, move_mods_to_object_from_snapshot, BulkCancelState,
    MoveModsToObjectInput,
};
use crate::modules::mutation::api::StepSettlement;
use crate::modules::mutation::coordinator::{IntentTarget, MutationCoordinator};
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
    intent_revision: Option<u64>,
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
    if !matches!(input.action, WorkspaceExplorerBulkAction::Toggle { .. }) {
        crate::modules::workspace::application::explorer::listing::validate_workspace_explorer_selection_identities(
            &resolved.expected_identities,
        )?;
    }
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
                intent_revision,
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
    intent_revision: Option<u64>,
) -> Result<WorkspaceSwitchResult, AppError> {
    execute_workspace_switch_request(app, WorkspaceSwitchRequest::Single(input), intent_revision)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn execute_workspace_object_bulk_switch(
    app: tauri::AppHandle,
    game_id: String,
    object_ids: Vec<String>,
    desired_enabled: bool,
    intent_revision: Option<u64>,
) -> Result<WorkspaceSwitchResult, AppError> {
    execute_workspace_switch_request(
        app,
        WorkspaceSwitchRequest::ObjectBatch {
            game_id,
            object_ids,
            desired_enabled,
        },
        intent_revision,
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
    fn intent_targets(&self) -> Vec<IntentTarget> {
        match self {
            Self::Single(input) => match input.target.kind {
                crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ModPath => {
                    vec![IntentTarget::ModPath(input.target.value.clone())]
                }
                crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ObjectId => {
                    vec![IntentTarget::ObjectId(input.target.value.clone())]
                }
            },
            Self::ObjectBatch { object_ids, .. } => object_ids
                .iter()
                .cloned()
                .map(IntentTarget::ObjectId)
                .collect(),
        }
    }

    fn is_leaf_toggle(&self) -> bool {
        matches!(self, Self::Single(input)
            if input.target.kind == crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ModPath
                && input.resolution == crate::modules::workspace::domain::workspace::WorkspaceSwitchResolution::Normal
                && !input.enable_disabled_ancestors
                && input.parent_enable_confirmation.is_none())
    }

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
    intent_revision: Option<u64>,
) -> Result<WorkspaceSwitchResult, AppError> {
    let config = app.state::<ConfigService>();
    let pool = app.state::<sqlx::SqlitePool>();
    let watcher_state = app.state::<WatcherState>();
    let disk_reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let op_lock = app.state::<MutationCoordinator>();
    let started_at = Instant::now();
    let game_id = request.game_id().to_string();
    let admission = op_lock.admit_intents(&game_id, intent_revision, request.intent_targets());
    if !admission.is_current() {
        return Ok(superseded_switch_result());
    }
    let _foreground_intent = op_lock.inner_lock().foreground_intent();
    let lock_wait_started_at = Instant::now();
    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    if !admission.is_current() {
        return Ok(superseded_switch_result());
    }
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
    if !admission.is_current() {
        return Ok(superseded_switch_result());
    }
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
    let storage_fast_path = leaf_storage_fast_path(request.is_leaf_toggle(), &scope, &mods_root);
    let preflight_started_at = Instant::now();
    if !storage_fast_path {
        let trusted_preflight_scope = trusted_toggle_scope(&scope, &mods_root)
            && disk_reconcile_state.trusted_regional_mutation_allowed(&game_id, &mods_root);
        let preflight_guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::Reconciliation,
            )
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
    }
    let preflight_elapsed = preflight_started_at.elapsed();
    if !admission.is_current() {
        return Ok(superseded_switch_result());
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
    if !admission.is_current() {
        mutation_lease.begin_rollback()?;
        let settlements = scope
            .renames
            .iter()
            .map(|rename| (rename.sequence, StepSettlement::RolledBack))
            .collect::<Vec<_>>();
        mutation_lease.settle_steps(&settlements)?;
        mutation_lease.finish_rollback()?;
        return Ok(superseded_switch_result());
    }
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
    let disk_revision = match verify_switch_storage_outcome(&scope)
        .and_then(|()| mutation_lease.mark_disk_committed())
    {
        Ok(revision) => revision,
        Err(error) => {
            let rollback_started = mutation_lease.begin_rollback();
            let rollback_result = prepared.rollback(&watcher_state);
            if let Err(rollback_error) = rollback_result {
                let message = format!(
                    "Storage verification or commit failed: {error}; filesystem rollback failed: {rollback_error}"
                );
                disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
                let _ = mutation_lease.fail(message.clone());
                return Err(AppError::Io(message));
            }
            rollback_started?;
            for rename in &scope.renames {
                mutation_lease.mark_step_rolled_back(rename.sequence)?;
            }
            mutation_lease.finish_rollback()?;
            disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
            return Err(error);
        }
    };
    result.disk_revision = Some(disk_revision);
    if trusted_mutation.is_none() {
        disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
    }
    if let Err(error) = crate::modules::system::application::app::post_apply::reserve_overlay_sync_revision_for_request(
        &game_id,
        crate::modules::reconciliation::api::RuntimeSyncRequest::Full,
    ) {
        log::error!("Could not invalidate runtime publication after disk commit: {error}");
    }
    drop(mutation_lease);
    queue_toggle_projection(app.clone(), pool.inner().clone(), game_id);
    log::debug!(
        "workspace switch timing outcome=disk_committed preflight_ms={} lock_wait_ms={} prepare_ms={} journal_prepare_ms={} rename_ms={} total_ms={}",
        preflight_elapsed.as_millis(),
        lock_wait_elapsed.as_millis(),
        prepare_elapsed.as_millis(),
        journal_prepare_elapsed.as_millis(),
        execute_elapsed.as_millis(),
        started_at.elapsed().as_millis(),
    );
    Ok(result)
}

fn verify_switch_storage_outcome(
    scope: &crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope,
) -> Result<(), AppError> {
    for (index, rename) in scope.renames.iter().enumerate() {
        let mut final_path = rename.new_path.clone();
        for later in scope.renames.iter().skip(index + 1) {
            if let Ok(suffix) = final_path.strip_prefix(&later.old_path) {
                final_path = later.new_path.join(suffix);
            }
        }
        let expected = rename.expected_identity.as_deref().ok_or_else(|| {
            AppError::Validation("Switch rename has no source identity".to_string())
        })?;
        let actual = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&final_path);
        if actual.as_deref() != Some(expected) {
            return Err(AppError::Io(format!(
                "Switched folder did not settle at expected disk path: {}",
                final_path.display()
            )));
        }
    }
    Ok(())
}

fn superseded_switch_result() -> WorkspaceSwitchResult {
    use crate::modules::workspace::domain::workspace::{WorkspaceImpact, WorkspaceSwitchStatus};

    WorkspaceSwitchResult {
        status: WorkspaceSwitchStatus::Noop,
        primary_path: None,
        changed_folder_paths: Vec::new(),
        changed_object_ids: Vec::new(),
        duplicates: Vec::new(),
        parent_enable_requirement: None,
        impact: WorkspaceImpact {
            rewrites: Vec::new(),
            changed_object_ids: Vec::new(),
            changed_folder_paths: Vec::new(),
            refresh_scopes: Vec::new(),
            warnings: Vec::new(),
        },
        sync_warning: None,
        runtime_sync_generation: None,
        disk_revision: None,
    }
}

static RUNNING_SWITCH_PROJECTIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn is_toggle_projection_operation(
    operation: &crate::modules::mutation::journal::Operation,
    game_id: &str,
) -> bool {
    operation.game_id == game_id
        && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
}

#[derive(Clone, serde::Serialize)]
struct WorkspaceSwitchProjected {
    game_id: String,
    disk_revision: u64,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct WorkspaceSwitchSnapshot {
    game_id: String,
    source_epoch: String,
    disk_revision: u64,
    projected_revision: u64,
}

pub(crate) fn projection_source_epoch(
    config: &ConfigService,
    game_id: &str,
) -> Result<String, AppError> {
    let root = config
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
        &root,
    )
    .ok_or_else(|| AppError::Io(format!("Mods root is unavailable: {}", root.display())))
}

async fn read_projection_checkpoint(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
) -> Result<u64, AppError> {
    let revision = sqlx::query_scalar::<_, i64>(
        "SELECT projected_revision FROM workspace_projection_checkpoints WHERE game_id = ? AND source_epoch = ?",
    )
    .bind(game_id)
    .bind(source_epoch)
    .fetch_optional(pool)
    .await?;
    Ok(revision
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0))
}

async fn persist_projection_checkpoint(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
    revision: u64,
) -> Result<(), AppError> {
    let revision = i64::try_from(revision)
        .map_err(|_| AppError::Validation("Disk revision exceeds SQLite range".to_string()))?;
    sqlx::query(
        "INSERT INTO workspace_projection_checkpoints (game_id, source_epoch, projected_revision) VALUES (?, ?, ?) \
         ON CONFLICT(game_id, source_epoch) DO UPDATE SET projected_revision = MAX(projected_revision, excluded.projected_revision)",
    )
    .bind(game_id)
    .bind(source_epoch)
    .bind(revision)
    .execute(pool)
    .await?;
    Ok(())
}

async fn checkpoint_current_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    expected_epoch: &str,
    revision: Option<u64>,
) -> Result<(), AppError> {
    if let Some(revision) = revision {
        let source_epoch = projection_source_epoch(&app.state::<ConfigService>(), game_id)?;
        if source_epoch != expected_epoch {
            return Err(AppError::Io(
                "Mods root changed during projection; retrying against current storage".to_string(),
            ));
        }
        persist_projection_checkpoint(pool, game_id, &source_epoch, revision).await?;
    }
    Ok(())
}

pub(crate) async fn complete_reconciled_toggle_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
    expected_epoch: &str,
    pending_ids: &[String],
    revision: Option<u64>,
) -> Result<(), AppError> {
    checkpoint_current_projection(app, pool, game_id, expected_epoch, revision).await?;
    coordinator.complete_disk_projection(pending_ids)?;
    publish_completed_toggle_projection(app, pool, game_id, revision);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_switch_snapshot(
    game_id: String,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    coordinator: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchSnapshot, AppError> {
    let source_epoch = projection_source_epoch(config.inner(), &game_id)?;
    let projected_revision =
        read_projection_checkpoint(pool.inner(), &game_id, &source_epoch).await?;
    let disk_revision = coordinator.latest_toggle_disk_revision(&game_id)?;
    Ok(WorkspaceSwitchSnapshot {
        game_id,
        source_epoch,
        disk_revision,
        projected_revision,
    })
}

#[tauri::command]
#[specta::specta]
pub fn admit_workspace_switch_intent(
    game_id: String,
    targets: Vec<crate::modules::workspace::domain::workspace::WorkspaceSwitchTarget>,
    intent_revision: u64,
    config: State<'_, ConfigService>,
    coordinator: State<'_, MutationCoordinator>,
) -> Result<bool, AppError> {
    if config.mods_root_for(&game_id).is_none() {
        return Err(AppError::NotFound("Game mods path not found".to_string()));
    }
    if targets.is_empty() || targets.len() > 10_000 {
        return Err(AppError::Validation(
            "Switch admission requires 1 to 10000 targets".to_string(),
        ));
    }
    if targets
        .iter()
        .any(|target| target.value.is_empty() || target.value.len() > 32_768)
    {
        return Err(AppError::Validation("Invalid switch target".to_string()));
    }
    let targets = targets.into_iter().map(|target| match target.kind {
        crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ModPath => {
            IntentTarget::ModPath(target.value)
        }
        crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ObjectId => {
            IntentTarget::ObjectId(target.value)
        }
    });
    Ok(coordinator
        .admit_intents(&game_id, Some(intent_revision), targets)
        .is_current())
}

fn running_switch_projections() -> &'static Mutex<HashSet<String>> {
    RUNNING_SWITCH_PROJECTIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(crate) fn publish_completed_toggle_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    disk_revision: Option<u64>,
) {
    if disk_revision.is_none() {
        return;
    }
    crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
        app,
        pool,
        game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
        crate::modules::reconciliation::api::RuntimeSyncRequest::Full,
    );
    if let Some(disk_revision) = disk_revision {
        if let Err(error) = app.emit(
            "workspace_switch:projected",
            WorkspaceSwitchProjected {
                game_id: game_id.to_string(),
                disk_revision,
            },
        ) {
            log::warn!("Could not emit workspace projection revision for '{game_id}': {error}");
        }
    }
}

pub(crate) fn queue_toggle_projection(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
) {
    let mut running = crate::shared::sync::lock(running_switch_projections());
    if !running.insert(game_id.clone()) {
        return;
    }
    drop(running);
    tauri::async_runtime::spawn(async move {
        run_workspace_switch_projection(&app, &pool, &game_id).await;
    });
}

async fn run_workspace_switch_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) {
    let mut retry_delay = Duration::from_millis(75);
    loop {
        tokio::time::sleep(retry_delay).await;
        let coordinator = app.state::<MutationCoordinator>();
        let state = app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let game_guard = state.game_lock(game_id).lock_owned().await;
        let Some(operation_guard) = coordinator.inner_lock().try_acquire_for_reconcile() else {
            drop(game_guard);
            retry_delay = Duration::from_millis(75);
            continue;
        };
        let lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_reconcile_guard(game_guard, operation_guard);
        let pending = match coordinator.pending_disk_commits() {
            Ok(pending) => pending
                .into_iter()
                .filter(|operation| is_toggle_projection_operation(operation, game_id))
                .collect::<Vec<_>>(),
            Err(error) => {
                log::error!(
                    "Could not inspect pending workspace projection for '{game_id}': {error}"
                );
                drop(lease);
                retry_delay = (retry_delay * 2).min(Duration::from_secs(5));
                continue;
            }
        };
        if pending.is_empty() {
            drop(lease);
            let mut running = crate::shared::sync::lock(running_switch_projections());
            match coordinator.pending_disk_commits() {
                Ok(remaining)
                    if !remaining
                        .iter()
                        .any(|operation| is_toggle_projection_operation(operation, game_id)) =>
                {
                    running.remove(game_id);
                    return;
                }
                Ok(_) => continue,
                Err(error) => {
                    log::error!(
                        "Could not settle workspace projection worker for '{game_id}': {error}"
                    );
                    retry_delay = Duration::from_secs(1);
                    continue;
                }
            }
        }
        let pending_ids = pending
            .iter()
            .map(|operation| operation.id.clone())
            .collect::<Vec<_>>();
        let projected_revision = pending
            .iter()
            .filter_map(|operation| operation.disk_revision)
            .max();
        let projection_epoch = match projection_source_epoch(&app.state::<ConfigService>(), game_id)
        {
            Ok(epoch) => epoch,
            Err(error) => {
                log::warn!("Workspace projection source is unavailable for '{game_id}': {error}");
                drop(lease);
                retry_delay = (retry_delay * 2).min(Duration::from_secs(5));
                continue;
            }
        };
        let authority_marker = app
            .try_state::<ConfigService>()
            .and_then(|config| config.mods_root_for(game_id))
            .and_then(|mods_root| {
                app.try_state::<WatcherState>()
                    .and_then(|watcher| watcher.current_session_for_root(&mods_root))
                    .and_then(|session| {
                        state
                            .authority_event_generation(game_id, session.generation())
                            .map(|generation| (mods_root, session.generation(), generation))
                    })
            });
        let trusted_authority =
            authority_marker
                .as_ref()
                .and_then(|(mods_root, watcher_session, _)| {
                    state.trusted_internal_mutation_evidence(game_id, mods_root, *watcher_session)
                });
        let changed_paths = pending
            .iter()
            .flat_map(|operation| &operation.steps)
            .flat_map(|step| [&step.old_path, &step.new_path])
            .flatten()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let scoped_paths = if pending.len() == 1 && pending[0].steps.len() == 1 {
            pending[0].steps[0]
                .old_path
                .as_ref()
                .zip(pending[0].steps[0].new_path.as_ref())
                .map(|(old, new)| {
                    vec![
                        old.to_string_lossy().into_owned(),
                        new.to_string_lossy().into_owned(),
                    ]
                })
        } else {
            None
        };
        let projection = tokio::select! {
            biased;
            _ = coordinator.inner_lock().wait_for_foreground_intent() => None,
            result = async {
                match scoped_paths {
                    Some(paths) => {
                        match crate::modules::reconciliation::application::disk_reconcile::emit::run_trusted_internal_disk_reconcile_under_lease(app, pool, game_id, paths, &lease).await {
                            Ok(result) if result.status.applied() => Ok(result),
                            Ok(_) | Err(_) => crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(app, pool, game_id, &lease).await,
                        }
                    },
                    None => crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(app, pool, game_id, &lease).await,
                }
            } => Some(result.and_then(require_applied_reconcile)),
        };
        let Some(projection) = projection else {
            drop(lease);
            retry_delay = Duration::from_millis(75);
            continue;
        };
        match projection {
            Ok(result) => {
                if let Some(authority) = &trusted_authority {
                    state.mark_trusted_internal_mutation_reconciled(
                        authority,
                        &result,
                        &changed_paths,
                    );
                } else if let Some((mods_root, watcher_session, observed_generation)) =
                    &authority_marker
                {
                    state.mark_authority_reconciled(
                        game_id,
                        mods_root,
                        *watcher_session,
                        *observed_generation,
                        &result,
                        &changed_paths,
                    );
                }
                let checkpoint = checkpoint_current_projection(
                    app,
                    pool,
                    game_id,
                    &projection_epoch,
                    projected_revision,
                )
                .await;
                if let Err(error) =
                    checkpoint.and_then(|()| coordinator.complete_disk_projection(&pending_ids))
                {
                    log::error!(
                        "Could not complete workspace disk projection for '{game_id}': {error}"
                    );
                    drop(lease);
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(5));
                    continue;
                }
                drop(lease);
                if let Err(error) = app.emit("disk_reconcile:result", &result) {
                    log::warn!(
                        "Could not emit completed workspace projection for '{game_id}': {error}"
                    );
                }
                publish_completed_toggle_projection(app, pool, game_id, projected_revision);
                retry_delay = Duration::from_millis(75);
            }
            Err(error) => {
                log::error!("Workspace disk projection remains pending for '{game_id}': {error}");
                if let Err(journal_error) =
                    coordinator.note_disk_projection_failure(&pending_ids, &error.to_string())
                {
                    log::error!("Could not record pending projection failure for '{game_id}': {journal_error}");
                }
                drop(lease);
                retry_delay = (retry_delay * 2).min(Duration::from_secs(5));
            }
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

fn leaf_storage_fast_path(
    is_leaf_toggle: bool,
    scope: &crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope,
    mods_root: &std::path::Path,
) -> bool {
    is_leaf_toggle && scope.renames.len() == 1 && trusted_toggle_scope(scope, mods_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_outcome_verifies_the_final_identity_after_chained_renames() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("DISABLED Blue");
        let middle = temp.path().join("Blue");
        let final_path = temp.path().join("DISABLED Blue Again");
        std::fs::create_dir(&first).unwrap();
        let identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&first).unwrap();
        let scope =
            crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope {
                renames: vec![
                crate::modules::workspace::application::workspace::switch::WorkspaceMutationRename {
                    sequence: 0,
                    old_path: first.clone(),
                    new_path: middle.clone(),
                    identity_path: first.clone(),
                    expected_identity: Some(identity.clone()),
                },
                crate::modules::workspace::application::workspace::switch::WorkspaceMutationRename {
                    sequence: 1,
                    old_path: middle.clone(),
                    new_path: final_path.clone(),
                    identity_path: first.clone(),
                    expected_identity: Some(identity),
                },
            ],
                changed_paths: Vec::new(),
                owning_roots: Vec::new(),
                touched_object_ids: Vec::new(),
            };
        std::fs::rename(&first, &middle).unwrap();
        std::fs::rename(&middle, &final_path).unwrap();
        verify_switch_storage_outcome(&scope).unwrap();
        std::fs::rename(&final_path, &middle).unwrap();
        assert!(verify_switch_storage_outcome(&scope).is_err());
    }

    #[tokio::test]
    async fn projection_checkpoint_is_monotonic_and_isolated_by_source_epoch() {
        let pool = crate::test_utils::init_test_db().await.pool;
        assert_eq!(
            read_projection_checkpoint(&pool, "game", "root-a")
                .await
                .unwrap(),
            0
        );
        persist_projection_checkpoint(&pool, "game", "root-a", 42)
            .await
            .unwrap();
        persist_projection_checkpoint(&pool, "game", "root-a", 17)
            .await
            .unwrap();
        assert_eq!(
            read_projection_checkpoint(&pool, "game", "root-a")
                .await
                .unwrap(),
            42
        );
        assert_eq!(
            read_projection_checkpoint(&pool, "game", "root-b")
                .await
                .unwrap(),
            0
        );
    }
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

    #[test]
    fn first_leaf_toggle_uses_storage_fast_path_without_watcher_authority() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        std::fs::create_dir(&old_path).expect("source folder");
        let scope = crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope {
            renames: vec![crate::modules::workspace::application::workspace::switch::WorkspaceMutationRename {
                sequence: 0,
                old_path: old_path.clone(),
                new_path: new_path.clone(),
                identity_path: old_path.clone(),
                expected_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_path),
            }],
            changed_paths: vec![
                old_path.to_string_lossy().into_owned(),
                new_path.to_string_lossy().into_owned(),
            ],
            owning_roots: vec!["DISABLED Blue".into(), "Blue".into()],
            touched_object_ids: Vec::new(),
        };
        assert!(leaf_storage_fast_path(true, &scope, temp.path()));
        assert!(!leaf_storage_fast_path(false, &scope, temp.path()));
    }
}
