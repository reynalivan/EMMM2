use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

use crate::domain::collection::{
    ApplyPreview, ApplyProgressSnapshot, ApplyResult, CollectionPreview, CollectionSummary,
    CreateCollectionInput, CreateCollectionMode, UpdateCollectionInput,
};
use crate::domain::errors::AppError;
use crate::domain::runtime_state::{CollectionRuntimeDescriptor, CollectionRuntimeSnapshot};
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::{collection_runtime, collection};

// ============================================================================
// Runtime state commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn get_collection_runtime_state(
    pool: State<'_, SqlitePool>,
    game_id: String,
) -> Result<CollectionRuntimeSnapshot, AppError> {
    let snapshot =
        collection_runtime::get_collection_runtime_state(pool.inner(), &game_id).await?;
    Ok(snapshot)
}

#[tauri::command]
#[specta::specta]
pub async fn get_collection_runtime_descriptor(
    pool: State<'_, SqlitePool>,
    game_id: String,
) -> Result<CollectionRuntimeDescriptor, AppError> {
    Ok(
        collection_runtime::get_collection_runtime_descriptor(pool.inner(), &game_id)
            .await?,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn get_apply_progress(
    game_id: String,
) -> Result<Option<ApplyProgressSnapshot>, AppError> {
    Ok(crate::services::apply_progress::get(&game_id))
}

// ============================================================================
// Collection Commands
// ============================================================================

async fn collection_preflight_paths(
    pool: &SqlitePool,
    collection_id: &str,
    mods_path: &Path,
) -> Result<Vec<String>, AppError> {
    let mods = crate::repo::collection::get_mods(pool, collection_id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let objects = crate::repo::collection::get_objects(pool, collection_id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let mut paths = BTreeSet::new();
    for relative_path in mods
        .into_iter()
        .map(|member| member.mod_path)
        .chain(objects.into_iter().filter_map(|member| member.path_key))
    {
        let path = PathBuf::from(relative_path);
        paths.insert(
            if path.is_absolute() {
                path
            } else {
                mods_path.join(path)
            }
            .to_string_lossy()
            .to_string(),
        );
    }
    Ok(paths.into_iter().collect())
}

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
    op_lock: State<'_, OperationLock>,
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
        crate::services::disk_reconcile::emit::ensure_mutation_preflight(
            &app,
            pool.inner(),
            &game_id,
        )
        .await?;
        Some(op_lock.acquire().await?)
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
    op_lock: State<'_, OperationLock>,
    game_id: String,
    name: String,
) -> Result<CollectionSummary, AppError> {
    crate::services::disk_reconcile::emit::ensure_mutation_preflight(&app, pool.inner(), &game_id)
        .await?;
    let _guard = op_lock.acquire().await?;
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
    config: State<'_, crate::services::config::ConfigService>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    disk_reconcile: State<'_, crate::services::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    collection_id: String,
    ignore_missing: Option<bool>,
) -> Result<ApplyResult, AppError> {
    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| {
            AppError::RuntimeState(crate::domain::errors::RuntimeStateError::GameNotFound {
                game_id: game_id.clone(),
            })
        })?;
    let mods_path = game.mod_path.clone();
    let preflight_paths =
        collection_preflight_paths(pool.inner(), &collection_id, &mods_path).await?;
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(&game_id, op_lock.inner())
        .await?;

    let result = collection::apply_collection(collection::ApplyCollectionRequest {
        pool: pool.inner(),
        game_id: &game_id,
        collection_id: &collection_id,
        capture_last_changes: true,
        mods_path,
        suppressor: watcher_state.suppressor.clone(),
        ignore_missing: ignore_missing.unwrap_or(false),
        settings,
    })
    .await?;

    drop(mutation_lease);
    Ok(result)
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
    op_lock: State<'_, OperationLock>,
    game_id: String,
    collection_id: String,
) -> Result<CollectionSummary, AppError> {
    crate::services::disk_reconcile::emit::ensure_mutation_preflight(&app, pool.inner(), &game_id)
        .await?;
    let operation_guard = op_lock.acquire().await?;
    let result = collection::replace_collection_with_current_state(
        pool.inner(),
        &game_id,
        &collection_id,
    )
    .await?;
    drop(operation_guard);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn save_collection_changes(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    collection_id: String,
    confirm_remove_missing: bool,
) -> Result<CollectionSummary, AppError> {
    crate::services::disk_reconcile::emit::ensure_mutation_preflight(&app, pool.inner(), &game_id)
        .await?;
    let _guard = op_lock.acquire().await?;
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
        return Err(crate::domain::errors::CollectionError::MissingMods {
            count: missing_paths.len(),
            paths: missing_paths,
        }
        .into());
    }
    Ok(collection::replace_collection_with_current_state(
        pool.inner(),
        &game_id,
        &collection_id,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_last_changes(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
) -> Result<(), AppError> {
    let _guard = op_lock.acquire().await?;
    collection::clear_last_changes(pool.inner(), &game_id).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn restore_last_changes(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    disk_reconcile: State<'_, crate::services::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
) -> Result<ApplyResult, AppError> {
    crate::services::disk_reconcile::emit::ensure_mutation_preflight(&app, pool.inner(), &game_id)
        .await?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(&game_id, op_lock.inner())
        .await?;
    let runtime = crate::repo::collection::runtime::get(pool.inner(), &game_id)
        .await?
        .ok_or_else(|| AppError::Validation("No Last changes snapshot exists".to_string()))?;
    let draft_id = runtime
        .draft_collection_id
        .ok_or_else(|| AppError::Validation("No Last changes snapshot exists".to_string()))?;
    let settings = config.get_settings();
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
    let result = collection::restore_collection_with_baseline(
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
    )
    .await?;
    drop(mutation_lease);
    let reconcile = crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
    )
    .await;
    Ok(settle_restore_reconcile(result, reconcile))
}

fn settle_restore_reconcile(
    mut result: ApplyResult,
    reconcile: Result<crate::services::disk_reconcile::types::DiskReconcileResult, AppError>,
) -> ApplyResult {
    let settlement = crate::services::disk_reconcile::emit::settle_committed_reconcile(reconcile);
    result.sync_warning = settlement.sync_warning;
    result
}

#[tauri::command]
#[specta::specta]
pub async fn delete_collection(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, OperationLock>,
    id: String,
) -> Result<(), AppError> {
    let _guard = op_lock.inner().acquire().await?;
    collection::delete_collection(pool.inner(), &id).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_collection_preview(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
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
    config: State<'_, crate::services::config::ConfigService>,
    game_id: String,
    collection_id: String,
) -> Result<ApplyPreview, AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let result = collection::preview_apply(
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
pub async fn app_startup_check(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<crate::domain::task::PipelineTask>, AppError> {
    crate::services::recovery::get_startup_recovery_tasks(pool.inner()).await
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus two IPC fields.
pub async fn resolve_recovery_task(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    disk_reconcile: State<'_, crate::services::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, OperationLock>,
    task_id: String,
    action: crate::domain::task::RecoveryAction,
) -> Result<(), AppError> {
    if action == crate::domain::task::RecoveryAction::Ignore {
        return crate::services::recovery::resolve_recovery_task(
            crate::services::recovery::RecoveryTaskRequest {
                pool: pool.inner(),
                config: config.inner(),
                watcher_state: watcher_state.inner(),
                task_id: &task_id,
                action,
            },
        )
        .await;
    }

    let task = crate::repo::task::get_task_by_id(pool.inner(), &task_id)
        .await?
        .ok_or_else(|| AppError::Validation(format!("Task {task_id} not found")))?;
    crate::services::disk_reconcile::emit::ensure_mutation_preflight(
        &app,
        pool.inner(),
        &task.game_id,
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(&task.game_id, op_lock.inner())
        .await?;

    crate::services::recovery::resolve_recovery_task(
        crate::services::recovery::RecoveryTaskRequest {
            pool: pool.inner(),
            config: config.inner(),
            watcher_state: watcher_state.inner(),
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
    use crate::domain::collection::ApplyResult;
    use crate::domain::errors::AppError;
    use crate::services::disk_reconcile::types::CommittedMutationSyncWarningKind;

    #[test]
    fn normal_apply_command_has_no_fallible_reconcile_after_service_commit() {
        let source = include_str!("cmds.rs");
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
    fn apply_command_uses_scoped_member_preflight_before_its_inline_projection() {
        let source = include_str!("cmds.rs");
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
            apply_command.contains("ensure_mutation_preflight_for_paths"),
            "collection apply must preflight only collection member roots"
        );
        assert!(
            apply_command.contains("collection_preflight_paths"),
            "the target member/object paths must define the scoped preflight"
        );
        assert!(
            !apply_command.contains("ensure_mutation_preflight(&app"),
            "collection apply must not perform a duplicate full preflight before inline projection"
        );
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
        let source = include_str!("cmds.rs");
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
