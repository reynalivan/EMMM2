use std::time::{Duration, Instant};
use tauri::{Manager, State};

use crate::modules::library::api::{
    bulk_delete_mods_from_snapshot, bulk_pin_mods_from_snapshot, bulk_set_mod_safety_from_snapshot,
    bulk_toggle_favorite_from_snapshot, bulk_toggle_mods_from_snapshot,
    bulk_update_info_from_snapshot, move_mods_to_object_from_snapshot, BulkCancelState,
    MoveModsToObjectInput,
};
use crate::modules::mutation::api::StepSettlement;
use crate::modules::mutation::coordinator::{IntentTarget, MutationCoordinator};
use crate::modules::reconciliation::api::{
    ensure_projection_epoch, projection_source_epoch, queue_toggle_projection,
};
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

const SLOW_WORKSPACE_SWITCH_THRESHOLD: Duration = Duration::from_millis(500);
const MAX_OBJECT_SWITCH_TARGETS: usize = 10_000;

struct ObjectSwitchOwnership {
    mods_root: std::path::PathBuf,
    root_proof: crate::platform::fs::file_utils::FilesystemIdentityProof,
    roots: std::collections::HashMap<
        String,
        (
            std::path::PathBuf,
            crate::platform::fs::file_utils::FilesystemIdentityProof,
        ),
    >,
}

impl ObjectSwitchOwnership {
    fn validate_request(
        &self,
        request: &WorkspaceSwitchRequest,
        config: &ConfigService,
    ) -> Result<(), AppError> {
        self.root_proof.validate(&self.mods_root)?;
        let configured_root = config
            .mods_root_for(request.game_id())
            .ok_or_else(|| AppError::NotFound("Game mods path not found".into()))?;
        self.root_proof.validate(&configured_root)?;
        for id in request.object_ids().unwrap_or_default() {
            let (path, proof) = self
                .roots
                .get(&id)
                .ok_or(AppError::ExplorerSnapshotExpired)?;
            crate::modules::workspace::application::workspace::switch::resolve_expected_switch_path(
                &self.mods_root, path, Some(proof.identity()),
            )?;
        }
        Ok(())
    }

    fn validate_prepared(
        &self,
        prepared: &crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch,
    ) -> Result<(), AppError> {
        use crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch;
        let validate_object =
            |object: &crate::modules::library::api::mods::object_switch::PreparedObjectSwitch| {
                let (_, proof) = self
                    .roots
                    .get(object.object_id())
                    .ok_or(AppError::ExplorerSnapshotExpired)?;
                proof.validate(object.current_path())
            };
        match prepared {
            PreparedWorkspaceSwitch::Object(object) => validate_object(object),
            PreparedWorkspaceSwitch::Objects(objects) => {
                objects.iter().try_for_each(validate_object)
            }
            _ => Err(AppError::Internal(
                "Object admission produced a non-object switch plan".into(),
            )),
        }
    }
}

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
    workspace.runtime.recovery_status = workspace_recovery_status(
        recovery_readiness,
        disk_reconcile_state
            .ensure_core_recovery_allows_preflight(&game_id)
            .is_ok(),
    );
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
    has_applied_core: bool,
) -> crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;
    use crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus;

    match readiness {
        InitialRecoveryReadiness::Ready { .. } => WorkspaceRecoveryStatus::Ready,
        InitialRecoveryReadiness::Failed { .. } if has_applied_core => {
            WorkspaceRecoveryStatus::Ready
        }
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

#[derive(Clone)]
enum WorkspaceSwitchRequest {
    Single(WorkspaceSwitchInput),
    ObjectBatch {
        game_id: String,
        object_ids: Vec<String>,
        desired_enabled: bool,
    },
}

impl WorkspaceSwitchRequest {
    fn object_ids(&self) -> Option<Vec<String>> {
        match self {
            Self::Single(input) if input.target.kind == crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ObjectId => Some(vec![input.target.value.clone()]),
            Self::ObjectBatch { object_ids, .. } => Some(object_ids.clone()),
            _ => None,
        }
    }

    async fn capture_object_ownership(
        &self,
        config: &ConfigService,
        pool: &sqlx::SqlitePool,
    ) -> Result<Option<ObjectSwitchOwnership>, AppError> {
        let Some(mut object_ids) = self.object_ids() else {
            return Ok(None);
        };
        let mut seen = std::collections::HashSet::new();
        object_ids.retain(|id| seen.insert(id.clone()));
        if object_ids.is_empty() || object_ids.len() > MAX_OBJECT_SWITCH_TARGETS {
            return Err(AppError::Validation(format!(
                "Object switch requires 1 to {MAX_OBJECT_SWITCH_TARGETS} targets"
            )));
        }
        let mods_root = config
            .mods_root_for(self.game_id())
            .ok_or_else(|| AppError::NotFound("Game mods path not found".into()))?;
        let root_proof =
            crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&mods_root)?;
        let resolved =
            crate::modules::library::api::mods::object_switch::resolve_object_root_paths(
                pool,
                self.game_id(),
                &object_ids,
            )
            .await?;
        if let Some((_, database_root, _)) = resolved.first() {
            root_proof.validate(std::path::Path::new(database_root))?;
        }
        let mut roots = std::collections::HashMap::with_capacity(resolved.len());
        for (object, _, path) in resolved {
            let path = std::path::PathBuf::from(path);
            let proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&path)?;
            if let Self::Single(input) = self {
                if input
                    .target
                    .expected_identity
                    .as_deref()
                    .is_some_and(|expected| expected != proof.identity())
                {
                    return Err(AppError::ExplorerSnapshotExpired);
                }
            }
            roots.insert(object.id, (path, proof));
        }
        let ownership = ObjectSwitchOwnership {
            mods_root,
            root_proof,
            roots,
        };
        ownership.validate_request(self, config)?;
        Ok(Some(ownership))
    }
    fn retain_current_objects(
        &mut self,
        admission: &crate::modules::mutation::coordinator::IntentAdmission,
    ) -> bool {
        if let Self::ObjectBatch {
            game_id,
            object_ids,
            ..
        } = self
        {
            let before = object_ids.len();
            object_ids.retain(|id| admission.is_current_object(game_id, id));
            return before != object_ids.len();
        }
        false
    }
    fn validate_admitted_target(
        &self,
        admission: &crate::modules::mutation::coordinator::IntentAdmission,
        prepared: &crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch,
    ) -> Result<(), AppError> {
        if let Self::Single(input) = self {
            if input.target.kind
                == crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ObjectId
            {
                if let crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch::Object(object) = prepared {
                    if input.target.expected_identity.as_deref().is_some_and(|expected| crate::platform::fs::file_utils::filesystem_identity(object.current_path()).as_deref() != Some(expected)) {
                        return Err(AppError::ExplorerSnapshotExpired);
                    }
                }
            }
            if input.target.kind
                == crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ModPath
            {
                if let Some(resolved) = prepared.resolved_mod_path() {
                    if input
                        .target
                        .expected_identity
                        .as_deref()
                        .is_some_and(|expected| {
                            crate::platform::fs::file_utils::filesystem_identity(
                                std::path::Path::new(resolved),
                            )
                            .as_deref()
                                != Some(expected)
                        })
                    {
                        return Err(AppError::ExplorerSnapshotExpired);
                    }
                    admission.validate_resolved_path(
                        std::path::Path::new(&input.target.value),
                        std::path::Path::new(resolved),
                    )?;
                }
            }
        }
        Ok(())
    }
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

    fn is_direct_prefix_toggle(&self) -> bool {
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
    mut request: WorkspaceSwitchRequest,
    intent_revision: Option<u64>,
) -> Result<WorkspaceSwitchResult, AppError> {
    let command_started_at = Instant::now();
    let source_epoch = projection_source_epoch(&app.state::<ConfigService>(), request.game_id())?;
    let object_ownership = request
        .capture_object_ownership(
            &app.state::<ConfigService>(),
            &app.state::<sqlx::SqlitePool>(),
        )
        .await?;
    loop {
        let result = execute_workspace_switch_request_once(
            app.clone(),
            request.clone(),
            intent_revision,
            &source_epoch,
            object_ownership.as_ref(),
            command_started_at,
        )
        .await?;
        if result.status
            != crate::modules::workspace::domain::workspace::WorkspaceSwitchStatus::Noop
        {
            return Ok(result);
        }
        let WorkspaceSwitchRequest::ObjectBatch { game_id, .. } = &request else {
            return Ok(result);
        };
        ensure_projection_epoch(&app.state::<ConfigService>(), game_id, &source_epoch)?;
        let admission = app.state::<MutationCoordinator>().admit_intents_in_epoch(
            game_id,
            Some(&source_epoch),
            intent_revision,
            request.intent_targets(),
        );
        if !request.retain_current_objects(&admission) {
            return Ok(result);
        }
        if matches!(&request, WorkspaceSwitchRequest::ObjectBatch { object_ids, .. } if object_ids.is_empty())
        {
            return Ok(result);
        }
        // Only rebuild a not-started plan. A disk-committed batch never returns
        // this superseded outcome and must finish before its successors.
    }
}

async fn execute_workspace_switch_request_once(
    app: tauri::AppHandle,
    mut request: WorkspaceSwitchRequest,
    intent_revision: Option<u64>,
    expected_epoch: &str,
    object_ownership: Option<&ObjectSwitchOwnership>,
    command_started_at: Instant,
) -> Result<WorkspaceSwitchResult, AppError> {
    let config = app.state::<ConfigService>();
    let pool = app.state::<sqlx::SqlitePool>();
    let watcher_state = app.state::<WatcherState>();
    let disk_reconcile_state = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    let op_lock = app.state::<MutationCoordinator>();
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    let trace_enabled = cfg!(debug_assertions) || diagnostics_enabled;
    let started_at = command_started_at;
    let game_id = request.game_id().to_string();
    if let WorkspaceSwitchRequest::Single(input) = &mut request {
        if input.target.kind
            == crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ModPath
        {
            let root = config
                .mods_root_for(&game_id)
                .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
            input.target.value = root
                .join(&input.target.value)
                .to_string_lossy()
                .into_owned();
            input.target.value = crate::modules::workspace::application::workspace::switch::resolve_expected_switch_path(
                &root, std::path::Path::new(&input.target.value), input.target.expected_identity.as_deref(),
            )?.to_string_lossy().into_owned();
        }
    }
    let admission_started_at = Instant::now();
    ensure_projection_epoch(config.inner(), &game_id, expected_epoch)?;
    let source_epoch = expected_epoch.to_string();
    let admission = op_lock.admit_intents_in_epoch(
        &game_id,
        Some(&source_epoch),
        intent_revision,
        request.intent_targets(),
    );
    let admission_elapsed = admission_started_at.elapsed();
    if !admission.is_current() {
        return Ok(superseded_switch_result());
    }
    let _foreground_intent = op_lock.inner_lock().foreground_intent();
    let storage_proof = if request.is_direct_prefix_toggle() {
        let WorkspaceSwitchRequest::Single(input) = &request else {
            unreachable!("normal mod path request");
        };
        let root = config
            .mods_root_for(&game_id)
            .ok_or_else(|| AppError::NotFound("Game mods path not found".into()))?;
        disk_reconcile_state.capture_toggle_storage_proof(
            &game_id,
            &root,
            &[std::path::PathBuf::from(&input.target.value)],
            watcher_state.inner(),
        )?
    } else {
        None
    };
    let lock_wait_started_at = Instant::now();
    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    ensure_projection_epoch(config.inner(), &game_id, &source_epoch)?;
    if !admission.is_current() {
        return Ok(superseded_switch_result());
    }
    if let Some(ownership) = object_ownership {
        ownership.validate_request(&request, config.inner())?;
    }
    let lock_wait_elapsed = lock_wait_started_at.elapsed();
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_recovery_allows_preflight(
        &app,
        &game_id,
    )?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let prepare_started_at = Instant::now();
    let mut prepared = request.prepare(config.inner(), pool.inner()).await?;
    request.validate_admitted_target(&admission, &prepared)?;
    if let Some(ownership) = object_ownership {
        ownership.validate_prepared(&prepared)?;
    }
    if !admission.is_current() {
        return Ok(superseded_switch_result());
    }
    let prepare_elapsed = prepare_started_at.elapsed();
    if let Some(result) = prepared.immediate_result() {
        if diagnostics_enabled {
            log::debug!(
                "workspace switch span outcome=immediate game_id={} source_epoch={} intent_revision={:?} admission_us={} game_lease_wait_us={} prepare_us={} total_us={}",
                game_id,
                source_epoch,
                intent_revision,
                admission_elapsed.as_micros(),
                lock_wait_elapsed.as_micros(),
                prepare_elapsed.as_micros(),
                started_at.elapsed().as_micros(),
            );
        }
        return Ok(result);
    }
    let mut scope = prepared.mutation_scope(&mods_root)?;
    let storage_proof = storage_proof.filter(|_| {
        direct_prefix_storage_fast_path(request.is_direct_prefix_toggle(), &scope, &mods_root)
    });
    if let Some(proof) = &storage_proof {
        disk_reconcile_state.validate_toggle_storage_proof(
            &game_id,
            &mods_root,
            &scope.changed_paths,
            proof,
            watcher_state.inner(),
        )?;
        prepared.set_mod_namespace_proof(proof.namespace());
    }
    if scope.renames.is_empty() {
        disk_reconcile_state.ensure_core_recovery_allows_preflight(&game_id)?;
        let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
            )
            .await?;
        let execute_started_at = Instant::now();
        if let Some(ownership) = object_ownership {
            ownership.validate_request(&request, config.inner())?;
            ownership.validate_prepared(&prepared)?;
        }
        let result = prepared.execute(&app, &watcher_state);
        if diagnostics_enabled {
            log::debug!(
                "workspace switch span outcome=exempt game_id={} source_epoch={} intent_revision={:?} admission_us={} game_lease_wait_us={} prepare_us={} execute_us={} total_us={}",
                game_id,
                source_epoch,
                intent_revision,
                admission_elapsed.as_micros(),
                lock_wait_elapsed.as_micros(),
                prepare_elapsed.as_micros(),
                execute_started_at.elapsed().as_micros(),
                started_at.elapsed().as_micros(),
            );
        }
        return result;
    }
    scope.validate_identities()?;
    let storage_fast_path =
        direct_prefix_storage_fast_path(request.is_direct_prefix_toggle(), &scope, &mods_root)
            && (storage_proof.is_some()
                || disk_reconcile_state.toggle_scope_is_ready(
                    &game_id,
                    &mods_root,
                    &scope.changed_paths,
                ));
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

        if let Some(ownership) = object_ownership {
            ownership.validate_request(&request, config.inner())?;
        }
        prepared = request.prepare(config.inner(), pool.inner()).await?;
        request.validate_admitted_target(&admission, &prepared)?;
        if let Some(ownership) = object_ownership {
            ownership.validate_prepared(&prepared)?;
        }
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
            if let Some(ownership) = object_ownership {
                ownership.validate_request(&request, config.inner())?;
            }
            prepared = request.prepare(config.inner(), pool.inner()).await?;
            request.validate_admitted_target(&admission, &prepared)?;
            if let Some(ownership) = object_ownership {
                ownership.validate_prepared(&prepared)?;
            }
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
            disk_reconcile_state.ensure_core_recovery_allows_preflight(&game_id)?;
            let _guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::WorkspaceConfiguration,
            )
            .await?;
            if let Some(ownership) = object_ownership {
                ownership.validate_request(&request, config.inner())?;
                ownership.validate_prepared(&prepared)?;
            }
            return prepared.execute(&app, &watcher_state);
        }
        if !trusted_toggle_scope(&scope, &mods_root) && crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
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
    let trusted_scope_candidate = trusted_toggle_scope(&scope, &mods_root);
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
        .acquire_operation(
            crate::modules::mutation::api::OperationPlan::new(
                "workspace-switch",
                game_id.clone(),
                journal_steps,
            )
            .with_source_epoch(source_epoch.clone()),
        )
        .await?;
    let journal_prepare_elapsed = journal_started_at.elapsed();
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_toggle_scope_guard(
        disk_reconcile_state.inner(),
        &game_id,
        &mods_root,
        &scope.changed_paths,
        game_guard,
        operation_guard,
        storage_proof.as_ref().map(|proof| (proof, watcher_state.inner())),
        async |game_guard, operation_guard| {
            let preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_under_game_lock(
                &app,
                pool.inner(),
                &game_id,
                scope.changed_paths.clone(),
                trusted_toggle_scope(&scope, &mods_root)
                    && disk_reconcile_state.trusted_regional_mutation_allowed(&game_id, &mods_root),
                game_guard,
                operation_guard.op_guard(),
            ).await?;
            if !trusted_toggle_scope(&scope, &mods_root)
                && crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
                    &preflight.folder_conflicts,
                    &scope.changed_paths,
                )
            {
                return Err(crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error());
            }
            Ok(())
        },
    ).await?;
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
        .filter(|session| {
            storage_proof.as_ref().is_some_and(|proof| proof.session() == session)
                || disk_reconcile_state.trusted_toggle_session_is_ready(
                &game_id,
                &mods_root,
                &scope.changed_paths,
                session.generation(),
            )
        })
        .and_then(|session| {
            let renames = scope
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
                .collect::<Option<Vec<_>>>()?;
            Some((session, renames))
        });
    let expected_echo_evidence = trusted_mutation.as_ref().map(|(session, renames)| {
        watcher_state
            .suppressor
            .expect_rename_echoes(&game_id, session, renames.clone())
    });
    let storage_started_at = Instant::now();
    let execute_started_at = Instant::now();
    let ownership_validation = ensure_projection_epoch(config.inner(), &game_id, &source_epoch)
        .and_then(|()| {
            if let Some(ownership) = object_ownership {
                ownership.validate_request(&request, config.inner())?;
                ownership.validate_prepared(&prepared)?;
            }
            Ok(())
        });
    if let Err(error) = ownership_validation {
        if let Some(evidence) = &expected_echo_evidence {
            watcher_state
                .suppressor
                .discard_expected_rename_echoes(evidence);
        }
        mutation_lease.abort_unapplied()?;
        return Err(error);
    }
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
            if let Some(evidence) = &expected_echo_evidence {
                watcher_state
                    .suppressor
                    .discard_expected_rename_echoes(evidence);
            }
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
    let storage_durable_elapsed = storage_started_at.elapsed();
    result.disk_revision = Some(disk_revision);
    result.source_epoch = Some(source_epoch);
    if let Some(evidence) = &expected_echo_evidence {
        if !watcher_state
            .suppressor
            .commit_expected_rename_echoes(evidence)
        {
            disk_reconcile_state.invalidate_authority(&game_id, &mods_root);
        }
    }
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
    let diagnostic_operation_id = trace_enabled.then(|| {
        op_lock
            .pending_disk_commits()
            .ok()
            .and_then(|operations| {
                operations
                    .into_iter()
                    .find(|operation| operation.disk_revision == Some(disk_revision))
            })
            .map(|operation| operation.id)
            .unwrap_or_else(|| "unavailable".to_string())
    });
    queue_toggle_projection(app.clone(), pool.inner().clone(), game_id.clone());
    if trace_enabled {
        log::info!(
            "workspace switch span outcome=disk_committed game_id={} source_epoch={} intent_revision={:?} operation_id={} disk_revision={} admission_us={} game_lease_wait_us={} prepare_us={} preflight_us={} operation_lease_and_journal_us={} rename_us={} storage_durable_us={} total_us={}",
            game_id,
            result.source_epoch.as_deref().unwrap_or(expected_epoch),
            intent_revision,
            diagnostic_operation_id.as_deref().unwrap_or("unavailable"),
            disk_revision,
            admission_elapsed.as_micros(),
            lock_wait_elapsed.as_micros(),
            prepare_elapsed.as_micros(),
            preflight_elapsed.as_micros(),
            journal_prepare_elapsed.as_micros(),
            execute_elapsed.as_micros(),
            storage_durable_elapsed.as_micros(),
            started_at.elapsed().as_micros(),
        );
    }
    if diagnostics_enabled && started_at.elapsed() >= SLOW_WORKSPACE_SWITCH_THRESHOLD {
        log::info!(
            "slow workspace switch disk commit game_id={} source_epoch={} intent_revision={:?} operation_id={} disk_revision={} preflight_ms={} lock_wait_ms={} prepare_ms={} journal_prepare_ms={} apply_ms={} storage_durable_ms={} total_ms={}",
            game_id,
            result.source_epoch.as_deref().unwrap_or(expected_epoch),
            intent_revision,
            diagnostic_operation_id.as_deref().unwrap_or("unavailable"),
            disk_revision,
            preflight_elapsed.as_millis(),
            lock_wait_elapsed.as_millis(),
            prepare_elapsed.as_millis(),
            journal_prepare_elapsed.as_millis(),
            execute_elapsed.as_millis(),
            storage_durable_elapsed.as_millis(),
            started_at.elapsed().as_millis(),
        );
    }
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
        source_epoch: None,
        disk_revision: None,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn admit_workspace_switch_intent(
    game_id: String,
    targets: Vec<crate::modules::workspace::domain::workspace::WorkspaceSwitchTarget>,
    intent_revision: u64,
    config: State<'_, ConfigService>,
    coordinator: State<'_, MutationCoordinator>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<bool, AppError> {
    let source_epoch = projection_source_epoch(config.inner(), &game_id)?;
    let root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
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
    let expected_objects = targets.iter().filter_map(|target| {
        (target.kind == crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ObjectId)
            .then_some(target).and_then(|target| target.expected_identity.as_deref().map(|identity| (&target.value, identity)))
    }).collect::<Vec<_>>();
    if !expected_objects.is_empty() {
        let root_proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&root)?;
        let ids = expected_objects
            .iter()
            .map(|(id, _)| (*id).clone())
            .collect::<Vec<_>>();
        let resolved =
            crate::modules::library::api::mods::object_switch::resolve_object_root_paths(
                pool.inner(),
                &game_id,
                &ids,
            )
            .await?;
        if let Some((_, database_root, _)) = resolved.first() {
            root_proof.validate(std::path::Path::new(database_root))?;
        }
        let actual = resolved
            .into_iter()
            .map(|(object, _, path)| {
                (
                    object.id,
                    crate::platform::fs::file_utils::filesystem_identity(std::path::Path::new(
                        &path,
                    )),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        if expected_objects
            .iter()
            .any(|(id, expected)| actual.get(*id).and_then(Option::as_deref) != Some(*expected))
        {
            return Err(AppError::ExplorerSnapshotExpired);
        }
        root_proof.validate(&root)?;
        ensure_projection_epoch(config.inner(), &game_id, &source_epoch)?;
    }
    let targets = targets.into_iter().map(|target| -> Result<IntentTarget, AppError> { Ok(match target.kind {
        crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ModPath => {
            let path = crate::modules::workspace::application::workspace::switch::resolve_expected_switch_path(
                &root, &root.join(target.value), target.expected_identity.as_deref(),
            )?;
            IntentTarget::ModPath(path.to_string_lossy().into_owned())
        }
        crate::modules::workspace::domain::workspace::WorkspaceSwitchTargetKind::ObjectId => {
            IntentTarget::ObjectId(target.value)
        }
    }) }).collect::<Result<Vec<_>, AppError>>()?;
    Ok(coordinator
        .admit_intents_in_epoch(
            &game_id,
            Some(&source_epoch),
            Some(intent_revision),
            targets,
        )
        .is_current())
}

fn same_mutation_paths(left: &[String], right: &[String]) -> bool {
    let normalized = |paths: &[String]| {
        paths
            .iter()
            .map(|path| {
                crate::shared::path_key::exact_location_key_for_path(std::path::Path::new(path))
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
    let Ok(mods_root) = crate::shared::path_key::physical_namespace_path(mods_root) else {
        return false;
    };
    scope.renames.iter().all(|rename| {
        let (Ok(old), Ok(new)) = (
            crate::shared::path_key::physical_namespace_path(&rename.old_path),
            crate::shared::path_key::physical_namespace_path(&rename.new_path),
        ) else {
            return false;
        };
        old.starts_with(&mods_root)
            && new.starts_with(&mods_root)
            && old.parent() == new.parent()
            && crate::shared::path_key::folder_path_key(
                &old.strip_prefix(&mods_root)
                    .unwrap_or(&old)
                    .to_string_lossy(),
                None,
            ) == crate::shared::path_key::folder_path_key(
                &new.strip_prefix(&mods_root)
                    .unwrap_or(&new)
                    .to_string_lossy(),
                None,
            )
    })
}

fn direct_prefix_storage_fast_path(
    is_direct_prefix_toggle: bool,
    scope: &crate::modules::workspace::application::workspace::switch::WorkspaceMutationScope,
    mods_root: &std::path::Path,
) -> bool {
    is_direct_prefix_toggle && scope.renames.len() == 1 && trusted_toggle_scope(scope, mods_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn object_switch_rejects_listing_identity_replaced_during_game_lock_wait() {
        use crate::modules::games::domain::models::GameType;
        use crate::modules::workspace::domain::workspace::{
            WorkspaceSwitchOriginSurface, WorkspaceSwitchResolution, WorkspaceSwitchTarget,
            WorkspaceSwitchTargetKind,
        };
        use crate::test_utils::{
            insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
        };
        let pool = crate::test_utils::init_test_db().await.pool;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        let original = root.join("Alice");
        std::fs::create_dir_all(&original).unwrap();
        insert_test_game(
            &pool,
            &TestGameFixture {
                id: "game",
                name: "Game",
                game_type: GameType::GIMI,
                path: temp.path().to_str().unwrap(),
                mods_path: root.to_str(),
            },
        )
        .await
        .unwrap();
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id: "alice",
                game_id: "game",
                name: "Alice",
                folder_path: "Alice",
                object_type: "Character",
            },
        )
        .await
        .unwrap();
        let config = ConfigService::new_for_test_async(pool.clone()).await;
        let expected = crate::platform::fs::file_utils::filesystem_identity(&original).unwrap();
        let request = WorkspaceSwitchRequest::Single(WorkspaceSwitchInput {
            game_id: "game".into(),
            target: WorkspaceSwitchTarget {
                kind: WorkspaceSwitchTargetKind::ObjectId,
                value: "alice".into(),
                expected_identity: Some(expected),
            },
            desired_enabled: false,
            resolution: WorkspaceSwitchResolution::Normal,
            enable_disabled_ancestors: false,
            parent_enable_confirmation: None,
            origin_surface: WorkspaceSwitchOriginSurface::ObjectList,
        });
        let coordinator = MutationCoordinator::unconfigured();
        let ownership = request
            .capture_object_ownership(&config, &pool)
            .await
            .unwrap()
            .unwrap();
        let admission = coordinator.admit_intents("game", Some(1), request.intent_targets());
        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        let blocker = state.game_lock("game").lock_owned().await;
        let held_original = root.join("Original Alice");
        std::fs::rename(&original, &held_original).unwrap();
        std::fs::create_dir(&original).unwrap();
        std::fs::write(original.join("foreign-marker.txt"), b"replacement").unwrap();
        drop(blocker);
        let _game_guard = state.game_lock("game").lock_owned().await;
        assert!(
            ownership.validate_request(&request, &config).is_err(),
            "pre-wait ownership must reject before re-preparing the replacement"
        );
        let prepared = request.prepare(&config, &pool).await.unwrap();
        assert!(
            request
                .validate_admitted_target(&admission, &prepared)
                .is_err(),
            "queued object request must not bind a replacement folder"
        );
        assert!(original.join("foreign-marker.txt").is_file());
        assert!(held_original.is_dir());
        assert!(!root.join("DISABLED Alice").exists());
        assert!(
            request
                .capture_object_ownership(&config, &pool)
                .await
                .is_err(),
            "a supplied old listing proof must reject before a fresh logical admission can bind"
        );
    }

    #[tokio::test]
    async fn queued_object_batch_keeps_pre_wait_ownership_without_ui_proof() {
        use crate::modules::games::domain::models::GameType;
        use crate::test_utils::{
            insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
        };
        let pool = crate::test_utils::init_test_db().await.pool;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        for name in ["Alice", "Bob"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        insert_test_game(
            &pool,
            &TestGameFixture {
                id: "game",
                name: "Game",
                game_type: GameType::GIMI,
                path: temp.path().to_str().unwrap(),
                mods_path: root.to_str(),
            },
        )
        .await
        .unwrap();
        for (id, name) in [("alice", "Alice"), ("bob", "Bob")] {
            insert_test_object(
                &pool,
                &TestObjectFixture {
                    id,
                    game_id: "game",
                    name,
                    folder_path: name,
                    object_type: "Character",
                },
            )
            .await
            .unwrap();
        }
        let config = std::sync::Arc::new(ConfigService::new_for_test_async(pool.clone()).await);
        let mut request = WorkspaceSwitchRequest::ObjectBatch {
            game_id: "game".into(),
            object_ids: vec!["alice".into(), "bob".into()],
            desired_enabled: false,
        };
        let ownership = request
            .capture_object_ownership(&config, &pool)
            .await
            .unwrap()
            .unwrap();
        let coordinator = MutationCoordinator::unconfigured();
        let admission = coordinator.admit_intents("game", Some(1), request.intent_targets());
        let state = std::sync::Arc::new(crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new());
        let blocker = state.game_lock("game").lock_owned().await;
        let waiting_state = std::sync::Arc::clone(&state);
        let waiting_config = std::sync::Arc::clone(&config);
        let waiting_pool = pool.clone();
        let waiting_request = request.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let waiter = tokio::spawn(async move {
            started_tx.send(()).unwrap();
            let _guard = waiting_state.game_lock("game").lock_owned().await;
            let validation = ownership.validate_request(&waiting_request, &waiting_config);
            if validation.is_ok() {
                waiting_request
                    .prepare(&waiting_config, &waiting_pool)
                    .await
                    .unwrap();
            }
            (ownership, validation)
        });
        started_rx.await.unwrap();
        std::fs::rename(root.join("Alice"), root.join("Original Alice")).unwrap();
        std::fs::create_dir(root.join("Alice")).unwrap();
        std::fs::write(root.join("Alice/foreign-marker.txt"), b"replacement").unwrap();
        drop(blocker);
        let (ownership, validation) = waiter.await.unwrap();
        assert!(
            validation.is_err(),
            "bare DB object cards must preserve ownership captured before the queue wait"
        );
        coordinator.admit_intents("game", Some(2), [IntentTarget::ObjectId("alice".into())]);
        assert!(request.retain_current_objects(&admission));
        ownership.validate_request(&request, &config).unwrap();
        let prepared = request.prepare(&config, &pool).await.unwrap();
        ownership.validate_prepared(&prepared).unwrap();
        let crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch::Objects(objects) = prepared else { panic!("object batch"); };
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].object_id(), "bob");
        objects[0].execute(&WatcherState::new()).unwrap();
        assert!(root.join("DISABLED Bob").is_dir());
        ownership.validate_request(&request, &config).unwrap();
        if let WorkspaceSwitchRequest::ObjectBatch {
            desired_enabled, ..
        } = &mut request
        {
            *desired_enabled = true;
        }
        let enabled_plan = request.prepare(&config, &pool).await.unwrap();
        ownership.validate_prepared(&enabled_plan).unwrap();
        let crate::modules::workspace::application::workspace::switch::PreparedWorkspaceSwitch::Objects(enabled_objects) = enabled_plan else { panic!("object batch"); };
        enabled_objects[0].execute(&WatcherState::new()).unwrap();
        assert!(
            root.join("Bob").is_dir(),
            "same-identity prefix renames remain valid for the latest intent"
        );
        assert!(root.join("Alice/foreign-marker.txt").is_file());
        assert!(root.join("Original Alice").is_dir());
        assert!(!root.join("DISABLED Alice").exists());
    }

    #[test]
    fn superseded_object_batch_preserves_unrelated_participants() {
        let coordinator = MutationCoordinator::unconfigured();
        let admission = coordinator.admit_intents(
            "game",
            Some(1),
            [
                IntentTarget::ObjectId("A".into()),
                IntentTarget::ObjectId("B".into()),
            ],
        );
        coordinator.admit_intents("game", Some(2), [IntentTarget::ObjectId("A".into())]);
        let mut request = WorkspaceSwitchRequest::ObjectBatch {
            game_id: "game".into(),
            object_ids: vec!["A".into(), "B".into()],
            desired_enabled: true,
        };
        assert!(request.retain_current_objects(&admission));
        let WorkspaceSwitchRequest::ObjectBatch { object_ids, .. } = request else {
            panic!("batch expected")
        };
        assert_eq!(object_ids, ["B"]);
    }

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

    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::InitialRecoveryReadiness;

    #[test]
    fn pending_recovery_maps_to_syncing_workspace_runtime() {
        assert_eq!(
            workspace_recovery_status(InitialRecoveryReadiness::Syncing { generation: 7 }, true),
            crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus::Syncing
        );
    }

    #[test]
    fn completed_index_does_not_lock_workspace_during_repairable_watcher_drift() {
        use crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus;

        assert_eq!(
            workspace_recovery_status(InitialRecoveryReadiness::Failed { generation: 7 }, true),
            WorkspaceRecoveryStatus::Ready
        );
        assert_eq!(
            workspace_recovery_status(InitialRecoveryReadiness::Failed { generation: 7 }, false),
            WorkspaceRecoveryStatus::Failed
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
        let old_path = std::fs::canonicalize(&old_path).expect("guard-derived source spelling");
        let new_path = old_path
            .parent()
            .unwrap()
            .join(new_path.file_name().unwrap());
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
        assert!(direct_prefix_storage_fast_path(true, &scope, temp.path()));
        assert!(!direct_prefix_storage_fast_path(false, &scope, temp.path()));
    }
}
