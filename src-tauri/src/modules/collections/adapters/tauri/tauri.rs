use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, State};

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
        ensure_current_runtime_snapshot_preflight(&app, pool.inner(), &game_id).await?;
        Some(
            op_lock
                .acquire_exempt(
                    crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
                )
                .await?,
        )
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
    ensure_current_runtime_snapshot_preflight(&app, pool.inner(), &game_id).await?;
    let _guard = op_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
        )
        .await?;
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
    let mutation_lease = disk_reconcile
        .acquire_nested_mutation_lease(&game_id, op_lock.inner())
        .await?;

    let result = collection::apply_collection_durable(
        collection::ApplyCollectionRequest {
            pool: pool.inner(),
            game_id: &game_id,
            collection_id: &collection_id,
            capture_last_changes: true,
            mods_path,
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: ignore_missing.unwrap_or(false),
            settings,
        },
        op_lock.inner(),
    )
    .await;

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
    ensure_current_runtime_snapshot_preflight(&app, pool.inner(), &game_id).await?;
    let operation_guard = op_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
        )
        .await?;
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
    ensure_current_runtime_snapshot_preflight(&app, pool.inner(), &game_id).await?;
    let _guard = op_lock
        .acquire_exempt(
            crate::modules::mutation::coordinator::MutationExemption::CollectionMetadata,
        )
        .await?;
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
    let started_at = std::time::Instant::now();
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &game_id,
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_nested_mutation_lease(&game_id, op_lock.inner())
        .await?;
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
            mods_path: game.mod_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: false,
            settings: settings.clone(),
        },
        restored_baseline,
        op_lock.inner(),
    )
    .await?;
    drop(mutation_lease);
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
    )
    .await;
    let result = settle_restore_reconcile(result, reconcile);
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

fn settle_restore_reconcile(
    mut result: ApplyResult,
    reconcile: Result<
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
        AppError,
    >,
) -> ApplyResult {
    let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(reconcile);
    result.sync_warning = settlement.sync_warning;
    result
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

    let result =
        collection::preview_apply(pool.inner(), &game_id, &collection_id, mods_path.as_deref())
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
    drop(mutation_lease);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::modules::collections::domain::collection::ApplyResult;
    use crate::modules::reconciliation::application::disk_reconcile::types::{
        CommittedMutationSyncWarningKind, FolderNameConflictCandidate, FolderNameConflictGroup,
    };
    use crate::shared::errors::AppError;

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
    fn current_runtime_snapshot_commands_share_scoped_preflight() {
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
                remainder[..end].contains("ensure_current_runtime_snapshot_preflight"),
                "{command} must scope folder-conflict blocking to active snapshot roots"
            );
        }
    }

    #[test]
    fn restore_reconcile_failure_is_returned_as_committed_success_warning() {
        let result = ApplyResult {
            mods_enabled: 1,
            mods_disabled: 0,
            warnings: Vec::new(),
            final_state_name: Some("Last changes".to_string()),
            partial_apply: false,
            skipped_missing_paths: Vec::new(),
            runtime_path_rewrites: Vec::new(),
            sync_warning: None,
        };

        let settled = super::settle_restore_reconcile(
            result,
            Err(AppError::Io("injected reconcile failure".to_string())),
        );

        let warning = settled
            .sync_warning
            .expect("committed restore must return a typed sync warning");
        assert_eq!(
            warning.kind,
            CommittedMutationSyncWarningKind::ReconcileFailed
        );
        assert!(warning.message.contains("injected reconcile failure"));
    }

    #[test]
    fn restore_command_does_not_turn_post_commit_reconcile_into_an_error() {
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

        assert!(restore_command.contains("settle_restore_reconcile(result, reconcile)"));
        let reconcile_call = restore_command
            .find("run_full_internal_disk_reconcile")
            .map(|offset| &restore_command[offset..])
            .expect("trailing reconcile call");
        let reconcile_await = reconcile_call
            .find(".await")
            .map(|offset| &reconcile_call[offset..offset + 7])
            .expect("trailing reconcile await");
        assert!(
            reconcile_await.ends_with(';'),
            "a committed restore must not return Err when its trailing reconcile fails"
        );
    }
}
