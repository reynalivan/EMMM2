use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::modules::collections::application::collection;
use crate::modules::collections::application::runtime as collection_runtime;
use crate::modules::collections::domain::collection::{
    ApplyPreview, ApplyProgressSnapshot, ApplyResult, CollectionPreview, CollectionSummary,
    CreateCollectionInput, CreateCollectionMode, UpdateCollectionInput,
};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::workspace::domain::runtime_state::{
    CollectionRuntimeDescriptor, CollectionRuntimeSnapshot,
};
use crate::shared::errors::AppError;

async fn record_collection_operation(
    app: &AppHandle,
    enabled: bool,
    operation: crate::modules::system::application::telemetry::TelemetryOperation,
    succeeded: bool,
    started_at: std::time::Instant,
) {
    if !enabled || !succeeded {
        return;
    }
    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .inner()
        .clone();
    let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
        operation,
        crate::modules::system::application::telemetry::TelemetryOutcome::Success,
        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
    )
    .with_duration(started_at.elapsed());
    let _ = telemetry
        .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
        .await;
}

fn collection_apply_changed_disk(result: &ApplyResult) -> bool {
    result.mods_enabled + result.mods_disabled > 0 || !result.runtime_path_rewrites.is_empty()
}

async fn ensure_current_runtime_snapshot_preflight(
    app: &AppHandle,
    pool: &SqlitePool,
    game_id: &str,
) -> Result<(), AppError> {
    let result = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_for_paths(
        app, pool, game_id, None,
    )
    .await?;
    if result.status
        == crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::AppliedWithFolderConflicts
    {
        let active_paths = collection::active_runtime_snapshot_scope_paths(pool, game_id).await?;
        if current_runtime_snapshot_conflicts_block(&result.folder_conflicts, &active_paths) {
            return Err(
                crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error(),
            );
        }
    }
    Ok(())
}

async fn acquire_current_snapshot_guard(
    app: &AppHandle,
    pool: &SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
) -> Result<CurrentSnapshotGuard, AppError> {
    let admission = crate::modules::mutation::api::admit_immutable_mutation(
        game_id,
        crate::modules::mutation::api::ImmutableMutationKind::CollectionCapture,
    )?;
    let mutation = acquire_current_snapshot_lease(app, pool, coordinator, game_id).await?;
    Ok(CurrentSnapshotGuard {
        _mutation: mutation,
        _admission: admission,
    })
}

pub(crate) async fn acquire_current_snapshot_lease(
    app: &AppHandle,
    pool: &SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    AppError,
> {
    let disk_reconcile = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    for _ in 0..3 {
        let source_epoch = crate::modules::reconciliation::api::projection_source_epoch(
            &app.state::<crate::modules::settings::application::config::ConfigService>(),
            game_id,
        )?;
        let pending_before = coordinator.pending_toggle_disk_commit_ids(game_id)?;
        ensure_current_runtime_snapshot_preflight(app, pool, game_id).await?;
        let guard = disk_reconcile
            .acquire_nested_mutation_lease(game_id, coordinator)
            .await?;
        let pending_after = coordinator.pending_toggle_disk_commit_ids(game_id)?;
        if pending_after.iter().all(|id| pending_before.contains(id)) {
            let projected_revision = coordinator
                .pending_disk_commits()?
                .into_iter()
                .filter(|operation| pending_after.contains(&operation.id))
                .filter_map(|operation| operation.disk_revision)
                .max();
            crate::modules::reconciliation::api::complete_reconciled_toggle_projection(
                app,
                pool,
                coordinator,
                game_id,
                &source_epoch,
                &pending_after,
                projected_revision,
            )
            .await?;
            return Ok(guard);
        }
        drop(guard);
    }
    Err(AppError::Io(
        "Mods changed repeatedly while capturing the collection; retry once switching settles"
            .to_string(),
    ))
}

struct CurrentSnapshotGuard {
    _mutation: crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    _admission: crate::modules::mutation::api::ImmutableMutationPermit,
}

fn current_runtime_snapshot_conflicts_block(
    conflicts: &[crate::modules::reconciliation::application::disk_reconcile::types::FolderNameConflictGroup],
    active_paths: &[String],
) -> bool {
    crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
        conflicts,
        active_paths,
    )
}

// ============================================================================
// Runtime state commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn get_collection_runtime_state(
    pool: State<'_, SqlitePool>,
    game_id: String,
) -> Result<CollectionRuntimeSnapshot, AppError> {
    let snapshot = collection_runtime::get_collection_runtime_state(pool.inner(), &game_id).await?;
    Ok(snapshot)
}

#[tauri::command]
#[specta::specta]
pub async fn get_collection_runtime_descriptor(
    pool: State<'_, SqlitePool>,
    game_id: String,
) -> Result<CollectionRuntimeDescriptor, AppError> {
    Ok(collection_runtime::get_collection_runtime_descriptor(pool.inner(), &game_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_apply_progress(
    game_id: String,
) -> Result<Option<ApplyProgressSnapshot>, AppError> {
    Ok(crate::modules::library::application::apply_progress::get(
        &game_id,
    ))
}

// ============================================================================
// Collection Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn list_collections(
    pool: State<'_, SqlitePool>,
    game_id: String,
) -> Result<Vec<CollectionSummary>, AppError> {
    let result = collection::list_collections(pool.inner(), &game_id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn create_collection(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    name: String,
    save_mode: Option<CreateCollectionMode>,
    source_collection_id: Option<String>,
) -> Result<CollectionSummary, AppError> {
    let captures_current_state = match save_mode.as_ref() {
        Some(CreateCollectionMode::CloneSnapshot) => false,
        Some(CreateCollectionMode::SaveCurrentState) => true,
        None => source_collection_id.is_none(),
    };
    let operation_guard = if captures_current_state {
        Some(acquire_current_snapshot_guard(&app, pool.inner(), op_lock.inner(), &game_id).await?)
    } else {
        None
    };
    let input = CreateCollectionInput {
        game_id,
        name,
        save_mode,
        source_collection_id,
    };

    let result = collection::create_collection(pool.inner(), input).await?;
    drop(operation_guard);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn save_current_runtime_as_collection(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    name: String,
) -> Result<CollectionSummary, AppError> {
    let _guard =
        acquire_current_snapshot_guard(&app, pool.inner(), op_lock.inner(), &game_id).await?;
    Ok(collection::create_collection(
        pool.inner(),
        CreateCollectionInput {
            game_id,
            name,
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary: states plus the IPC payload.
pub async fn apply_collection(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher_state: State<
        '_,
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    collection_id: String,
    ignore_missing: Option<bool>,
) -> Result<ApplyResult, AppError> {
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &game_id,
        crate::modules::mutation::api::ImmutableMutationKind::CollectionApply,
    )?;
    let started_at = std::time::Instant::now();
    let settings = config.get_settings();
    let diagnostics_enabled = settings.diagnostics.telemetry_enabled;
    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| {
            AppError::RuntimeState(crate::shared::errors::RuntimeStateError::GameNotFound {
                game_id: game_id.clone(),
            })
        })?;
    let mods_path = game.mod_path.clone();
    let root_proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&mods_path)?;
    let mut mutation_lease = None;
    for _ in 0..3 {
        let source_epoch = root_proof.identity();
        root_proof.validate(&mods_path)?;
        crate::modules::reconciliation::api::ensure_projection_epoch(
            config.inner(),
            &game_id,
            source_epoch,
        )?;
        let pending_before = op_lock.pending_toggle_disk_commit_ids(&game_id)?;
        if !pending_before.is_empty() {
            ensure_current_runtime_snapshot_preflight(&app, pool.inner(), &game_id).await?;
        }
        let preflight_paths = collection::collection_preflight_scope_paths(
            pool.inner(),
            &game_id,
            &collection_id,
            &mods_path,
        )
        .await?;
        crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
            &app,
            pool.inner(),
            &game_id,
            Some(&preflight_paths),
        )
        .await?;
        let lease = disk_reconcile
            .acquire_nested_mutation_lease(&game_id, op_lock.inner())
            .await?;
        root_proof.validate(&mods_path)?;
        crate::modules::reconciliation::api::ensure_projection_epoch(
            config.inner(),
            &game_id,
            source_epoch,
        )?;
        let pending_after = op_lock.pending_toggle_disk_commit_ids(&game_id)?;
        if pending_after.iter().all(|id| pending_before.contains(id)) {
            let projected_revision = op_lock
                .pending_disk_commits()?
                .into_iter()
                .filter(|operation| pending_after.contains(&operation.id))
                .filter_map(|operation| operation.disk_revision)
                .max();
            crate::modules::reconciliation::api::complete_reconciled_toggle_projection(
                &app,
                pool.inner(),
                op_lock.inner(),
                &game_id,
                source_epoch,
                &pending_after,
                projected_revision,
            )
            .await?;
            mutation_lease = Some(lease);
            break;
        }
        drop(lease);
    }
    let mutation_lease = mutation_lease.ok_or_else(|| {
        AppError::Io(
            "Mods changed repeatedly while applying the collection; retry once switching settles"
                .to_string(),
        )
    })?;

    let mut result = collection::apply_collection_durable(
        collection::ApplyCollectionRequest {
            pool: pool.inner(),
            game_id: &game_id,
            collection_id: &collection_id,
            capture_last_changes: true,
            mods_path: mods_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: ignore_missing.unwrap_or(false),
            settings,
        },
        op_lock.inner(),
    )
    .await;

    if let Ok(applied) = &mut result {
        if collection_apply_changed_disk(applied) {
            let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
                crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(
                    &app,
                    pool.inner(),
                    &game_id,
                    &mutation_lease,
                )
                .await,
            );
            if let Some(reconcile) = settlement.reconcile {
                if let Err(error) = app.emit("disk_reconcile:result", &reconcile) {
                    log::warn!("Could not emit collection disk reconcile result: {error}");
                }
            }
            applied.sync_warning = settlement.sync_warning;
        }
        crate::modules::reconciliation::api::enqueue_runtime_sync_for_rewrites(
            &app,
            pool.inner(),
            &game_id,
            &mods_path,
            crate::modules::reconciliation::api::RuntimeSyncCause::CollectionApplied,
            &applied.runtime_path_rewrites,
        );
    }

    drop(mutation_lease);
    record_collection_operation(
        &app,
        diagnostics_enabled,
        crate::modules::system::application::telemetry::TelemetryOperation::CollectionApply,
        result.is_ok(),
        started_at,
    )
    .await;
    result.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn update_collection(
    pool: State<'_, SqlitePool>,
    game_id: String,
    id: String,
    name: Option<String>,
) -> Result<CollectionSummary, AppError> {
    let input = UpdateCollectionInput { id, game_id, name };
    let result = collection::update_collection(pool.inner(), input).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn replace_collection_with_current_state(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    collection_id: String,
) -> Result<CollectionSummary, AppError> {
    let operation_guard =
        acquire_current_snapshot_guard(&app, pool.inner(), op_lock.inner(), &game_id).await?;
    let result =
        collection::replace_collection_with_current_state(pool.inner(), &game_id, &collection_id)
            .await?;
    drop(operation_guard);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn save_collection_changes(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    collection_id: String,
    confirm_remove_missing: bool,
) -> Result<CollectionSummary, AppError> {
    let _guard =
        acquire_current_snapshot_guard(&app, pool.inner(), op_lock.inner(), &game_id).await?;
    let mods_path = config
        .get_settings()
        .games
        .into_iter()
        .find(|game| game.id == game_id)
        .map(|game| game.mod_path.to_string_lossy().to_string());
    let preview = collection::get_collection_preview(
        pool.inner(),
        &game_id,
        &collection_id,
        mods_path.as_deref(),
    )
    .await?;
    let missing_paths = preview
        .projected_state
        .active_roots
        .iter()
        .filter(|root| root.is_missing)
        .map(|root| root.source_path.clone())
        .collect::<Vec<_>>();
    if !confirm_remove_missing && !missing_paths.is_empty() {
        return Err(crate::shared::errors::CollectionError::MissingMods {
            count: missing_paths.len(),
            paths: missing_paths,
        }
        .into());
    }
    Ok(
        collection::replace_collection_with_current_state(pool.inner(), &game_id, &collection_id)
            .await?,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn clear_last_changes(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
) -> Result<(), AppError> {
    let _guard = op_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
        )
        .await?;
    collection::clear_last_changes(pool.inner(), &game_id).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn restore_last_changes(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher_state: State<
        '_,
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
) -> Result<ApplyResult, AppError> {
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &game_id,
        crate::modules::mutation::api::ImmutableMutationKind::CollectionApply,
    )?;
    let started_at = std::time::Instant::now();
    let runtime =
        crate::modules::collections::adapters::sqlite::runtime::get(pool.inner(), &game_id)
            .await?
            .ok_or_else(|| AppError::Validation("No Last changes snapshot exists".to_string()))?;
    let draft_id = runtime
        .draft_collection_id
        .ok_or_else(|| AppError::Validation("No Last changes snapshot exists".to_string()))?;
    let settings = config.get_settings();
    let diagnostics_enabled = settings.diagnostics.telemetry_enabled;
    let game = settings
        .games
        .iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}' not found")))?;
    let mods_path = game.mod_path.clone();
    let root_proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&mods_path)?;
    let preflight_paths =
        collection::collection_preflight_scope_paths(pool.inner(), &game_id, &draft_id, &mods_path)
            .await?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_nested_mutation_lease(&game_id, op_lock.inner())
        .await?;
    root_proof.validate(&mods_path)?;
    crate::modules::reconciliation::api::ensure_projection_epoch(
        config.inner(),
        &game_id,
        root_proof.identity(),
    )?;
    let restored_baseline = collection::valid_active_baseline(
        pool.inner(),
        &game_id,
        runtime.draft_base_collection_id.as_deref(),
    )
    .await?;
    let result = collection::restore_collection_with_baseline_durable(
        collection::ApplyCollectionRequest {
            pool: pool.inner(),
            game_id: &game_id,
            collection_id: &draft_id,
            capture_last_changes: false,
            mods_path: mods_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: false,
            settings: settings.clone(),
        },
        restored_baseline,
        op_lock.inner(),
    )
    .await?;
    crate::modules::reconciliation::api::enqueue_runtime_sync_for_rewrites(
        &app,
        pool.inner(),
        &game_id,
        &mods_path,
        crate::modules::reconciliation::api::RuntimeSyncCause::CollectionApplied,
        &result.runtime_path_rewrites,
    );
    drop(mutation_lease);
    record_collection_operation(
        &app,
        diagnostics_enabled,
        crate::modules::system::application::telemetry::TelemetryOperation::Restore,
        true,
        started_at,
    )
    .await;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_collection(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    id: String,
) -> Result<(), AppError> {
    let _guard = op_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
        )
        .await?;
    collection::delete_collection(pool.inner(), &id).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_collection_preview(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    collection_id: String,
    game_id: String,
) -> Result<CollectionPreview, AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let result = collection::get_collection_preview(
        pool.inner(),
        &game_id,
        &collection_id,
        mods_path.as_deref(),
    )
    .await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn preview_apply_collection(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    game_id: String,
    collection_id: String,
) -> Result<ApplyPreview, AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let safe_mode_enabled = settings.safety.runtime_safe_mode_for(&game_id);
    let result = collection::preview_apply(
        pool.inner(),
        &game_id,
        &collection_id,
        mods_path.as_deref(),
        safe_mode_enabled,
    )
    .await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn app_startup_check(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<crate::modules::workspace::domain::task::PipelineTask>, AppError> {
    crate::modules::workspace::application::recovery::get_startup_recovery_tasks(pool.inner()).await
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus two IPC fields.
pub async fn resolve_recovery_task(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher_state: State<
        '_,
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    task_id: String,
    action: crate::modules::workspace::domain::task::RecoveryAction,
) -> Result<(), AppError> {
    if action == crate::modules::workspace::domain::task::RecoveryAction::Ignore {
        return crate::modules::workspace::application::recovery::resolve_recovery_task(
            crate::modules::workspace::application::recovery::RecoveryTaskRequest {
                pool: pool.inner(),
                config: config.inner(),
                watcher_state: watcher_state.inner(),
                coordinator: op_lock.inner(),
                task_id: &task_id,
                action,
            },
        )
        .await;
    }

    let task =
        crate::modules::workspace::adapters::sqlite::task::get_task_by_id(pool.inner(), &task_id)
            .await?
            .ok_or_else(|| AppError::Validation(format!("Task {task_id} not found")))?;
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &task.game_id,
        crate::modules::mutation::api::ImmutableMutationKind::Recovery,
    )?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &task.game_id,
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_nested_mutation_lease(&task.game_id, op_lock.inner())
        .await?;

    crate::modules::workspace::application::recovery::resolve_recovery_task(
        crate::modules::workspace::application::recovery::RecoveryTaskRequest {
            pool: pool.inner(),
            config: config.inner(),
            watcher_state: watcher_state.inner(),
            coordinator: op_lock.inner(),
            task_id: &task_id,
            action,
        },
    )
    .await?;
    crate::modules::reconciliation::api::enqueue_runtime_sync(
        &app,
        pool.inner(),
        &task.game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::Recovery,
    );
    drop(mutation_lease);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::collection_apply_changed_disk;
    use crate::modules::collections::domain::collection::ApplyResult;
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        FolderNameConflictCandidate, FolderNameConflictGroup,
    };

    #[test]
    fn object_only_collection_rename_requires_terminal_disk_authority() {
        let result = ApplyResult {
            mods_enabled: 0,
            mods_disabled: 0,
            warnings: Vec::new(),
            final_state_name: None,
            partial_apply: false,
            skipped_missing_paths: Vec::new(),
            runtime_path_rewrites: vec![
                crate::modules::workspace::domain::workspace::WorkspacePathRewrite {
                    old_path: "DISABLED Object".to_string(),
                    new_path: "Object".to_string(),
                },
            ],
            sync_warning: None,
        };
        assert!(collection_apply_changed_disk(&result));
    }

    #[test]
    fn normal_apply_command_has_no_fallible_reconcile_after_service_commit() {
        let source = include_str!("tauri.rs");
        let start = source
            .find("pub async fn apply_collection(")
            .expect("apply command source");
        let remainder = &source[start..];
        let end = remainder[1..]
            .find("#[tauri::command]")
            .map(|offset| offset + 1)
            .expect("next command boundary");
        let apply_command = &remainder[..end];

        assert!(
            !apply_command.contains("run_full_internal_disk_reconcile"),
            "a committed apply must not be converted to Err by an outer reconcile"
        );
        let durable_apply = apply_command
            .find("apply_collection_durable")
            .expect("durable collection apply");
        let runtime_queue = apply_command
            .find("enqueue_runtime_sync_for_rewrites")
            .expect("async runtime queue");
        assert!(
            durable_apply < runtime_queue,
            "runtime work must be queued only after the durable apply returns"
        );
    }

    #[test]
    fn current_runtime_snapshot_preflight_blocks_only_active_conflict_scopes() {
        let inactive_conflicts = vec![FolderNameConflictGroup {
            group_id: "blue".to_string(),
            identity: "ainoz/blue".to_string(),
            display_name: "Blue".to_string(),
            candidates: vec![FolderNameConflictCandidate {
                path: "E:/Mods/AINOZ/DISABLED Blue".to_string(),
                folder_name: "DISABLED Blue".to_string(),
                base_name: "Blue".to_string(),
                is_enabled: false,
            }],
        }];

        let active_paths = vec!["E:/Mods/AINOZ/Active".to_string()];
        assert!(!super::current_runtime_snapshot_conflicts_block(
            &inactive_conflicts,
            &active_paths,
        ));
        assert!(!super::current_runtime_snapshot_conflicts_block(
            &[],
            &active_paths,
        ));

        let active_conflicts = vec![FolderNameConflictGroup {
            group_id: "active-blue".to_string(),
            identity: "ainoz/blue".to_string(),
            display_name: "Blue".to_string(),
            candidates: vec![FolderNameConflictCandidate {
                path: "E:/Mods/AINOZ/Active".to_string(),
                folder_name: "Active".to_string(),
                base_name: "Active".to_string(),
                is_enabled: true,
            }],
        }];
        assert!(super::current_runtime_snapshot_conflicts_block(
            &active_conflicts,
            &active_paths,
        ));
    }

    #[test]
    fn current_runtime_snapshot_commands_share_projection_barrier() {
        let source = include_str!("tauri.rs");
        for command in [
            "pub async fn create_collection(",
            "pub async fn save_current_runtime_as_collection(",
            "pub async fn replace_collection_with_current_state(",
            "pub async fn save_collection_changes(",
        ] {
            let start = source.find(command).expect("collection command");
            let remainder = &source[start..];
            let end = remainder[1..]
                .find("#[tauri::command]")
                .map(|offset| offset + 1)
                .unwrap_or(remainder.len());
            assert!(
                remainder[..end].contains("acquire_current_snapshot_guard"),
                "{command} must reconcile pending disk changes before capturing current state"
            );
        }
    }

    #[test]
    fn current_snapshot_retains_game_and_operation_lease_during_capture() {
        let source = include_str!("tauri.rs");
        let start = source
            .find("async fn acquire_current_snapshot_guard(")
            .unwrap();
        let end = source[start..]
            .find("fn current_runtime_snapshot_conflicts_block(")
            .unwrap()
            + start;
        let guard = &source[start..end];
        assert!(guard.contains("acquire_nested_mutation_lease(game_id, coordinator)"));
        assert!(!guard.contains("acquire_exempt("));
        assert!(guard.contains("DiskMutationLease"));
    }

    #[test]
    fn immutable_collection_admission_precedes_projection_and_storage_waits() {
        let source = include_str!("tauri.rs");
        let helper_start = source
            .find("async fn acquire_current_snapshot_guard(")
            .unwrap();
        let helper_end = source[helper_start..]
            .find("struct CurrentSnapshotGuard")
            .unwrap()
            + helper_start;
        let helper = &source[helper_start..helper_end];
        assert!(
            helper.find("admit_immutable_mutation").unwrap()
                < helper
                    .find("ensure_current_runtime_snapshot_preflight")
                    .unwrap()
        );
        for command in [
            "pub async fn apply_collection(",
            "pub async fn restore_last_changes(",
        ] {
            let start = source.find(command).unwrap();
            let remainder = &source[start..];
            let end = remainder[1..]
                .find("#[tauri::command]")
                .map(|offset| offset + 1)
                .unwrap_or(remainder.len());
            let command_source = &remainder[..end];
            assert!(
                command_source.find("admit_immutable_mutation").unwrap()
                    < command_source
                        .find("acquire_nested_mutation_lease")
                        .unwrap()
            );
        }
    }

    #[test]
    fn restore_queues_runtime_sync_without_a_second_full_reconcile() {
        let source = include_str!("tauri.rs");
        let start = source
            .find("pub async fn restore_last_changes(")
            .expect("restore command source");
        let remainder = &source[start..];
        let end = remainder[1..]
            .find("#[tauri::command]")
            .map(|offset| offset + 1)
            .expect("next command boundary");
        let restore_command = &remainder[..end];

        assert!(
            !restore_command.contains("run_full_internal_disk_reconcile"),
            "the durable pipeline already reconciled the affected roots"
        );
        assert!(restore_command.contains("ensure_mutation_preflight_for_paths"));
        assert!(!restore_command.contains("ensure_mutation_preflight("));
        assert!(restore_command.contains("enqueue_runtime_sync_for_rewrites"));
    }
}
