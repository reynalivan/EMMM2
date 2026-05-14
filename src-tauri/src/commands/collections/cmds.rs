use sqlx::SqlitePool;
use tauri::State;

use crate::domain::collection::{
    ApplyPreview, ApplyProgressSnapshot, ApplyResult, CollectionPreview, CollectionSummary,
    CreateCollectionInput, CreateCollectionMode, UpdateCollectionInput,
};
use crate::domain::corridor::{CorridorSnapshot, CorridorSwitchPreview, SwitchResult};
use crate::domain::errors::AppError;
use crate::domain::pin::PinStatus;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::{collection_service, corridor_service, pin_service};

// ============================================================================
// Corridor Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn get_corridor_state(
    pool: State<'_, SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<CorridorSnapshot, AppError> {
    let snapshot = corridor_service::get_corridor_state(pool.inner(), &game_id, is_safe).await?;
    Ok(snapshot)
}

#[tauri::command]
#[specta::specta]
pub async fn get_apply_progress(
    config: State<'_, crate::services::config::ConfigService>,
    game_id: String,
) -> Result<Option<ApplyProgressSnapshot>, AppError> {
    let settings = config.get_settings();
    Ok(crate::services::apply_progress_service::get(
        &game_id,
        settings.safe_mode.enabled,
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn switch_corridor(
    pool: State<'_, SqlitePool>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    config: State<'_, crate::services::config::ConfigService>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    target_safe: bool,
) -> Result<SwitchResult, AppError> {
    let _guard = op_lock
        .inner()
        .acquire()
        .await
        .map_err(AppError::Internal)?;
    let settings = config.get_settings();
    if settings.safe_mode.enabled == target_safe {
        let snapshot =
            corridor_service::get_corridor_state(pool.inner(), &game_id, target_safe).await?;
        return Ok(SwitchResult {
            success: true,
            active_safe: target_safe,
            mods_disabled: 0,
            mods_restored: 0,
            new_signature: snapshot.current_signature,
            warnings: Vec::new(),
            restored_collection_id: snapshot.active_collection_id,
        });
    }

    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| {
            AppError::Corridor(crate::domain::errors::CorridorError::GameNotFound {
                game_id: game_id.clone(),
            })
        })?;

    let mods_path = game.mod_path.clone();

    let result = corridor_service::switch_corridor(
        pool.inner(),
        &game_id,
        target_safe,
        mods_path,
        watcher_state.suppressor.clone(),
        &watcher_state,
        settings,
    )
    .await?;
    config
        .set_safe_mode_enabled(result.active_safe)
        .map_err(AppError::Internal)?;

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn preview_corridor_switch(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    game_id: String,
    target_safe: bool,
) -> Result<CorridorSwitchPreview, AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let preview = corridor_service::preview_switch(
        pool.inner(),
        &game_id,
        settings.safe_mode.enabled,
        target_safe,
        mods_path.as_deref(),
    )
    .await?;
    Ok(preview)
}

// ============================================================================
// Collection Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn list_collections(
    pool: State<'_, SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<Vec<CollectionSummary>, AppError> {
    let result =
        collection_service::list_collections(pool.inner(), &game_id, is_safe, None).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn create_collection(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    game_id: String,
    name: String,
    save_mode: Option<CreateCollectionMode>,
    source_collection_id: Option<String>,
) -> Result<CollectionSummary, AppError> {
    let settings = config.get_settings();
    let is_safe = settings.safe_mode.enabled;

    let input = CreateCollectionInput {
        game_id,
        name,
        is_safe,
        save_mode,
        source_collection_id,
    };

    let result = collection_service::create_collection(pool.inner(), input).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn apply_collection(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    collection_id: String,
    ignore_missing: Option<bool>,
) -> Result<ApplyResult, AppError> {
    let _guard = op_lock
        .inner()
        .acquire()
        .await
        .map_err(AppError::Internal)?;
    let settings = config.get_settings();
    let is_safe = settings.safe_mode.enabled;
    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| {
            AppError::Corridor(crate::domain::errors::CorridorError::GameNotFound {
                game_id: game_id.clone(),
            })
        })?;
    let mods_path = game.mod_path.clone();

    let result = collection_service::apply_collection(collection_service::ApplyCollectionRequest {
        pool: pool.inner(),
        game_id: &game_id,
        collection_id: &collection_id,
        is_safe,
        mods_path,
        suppressor: watcher_state.suppressor.clone(),
        ignore_missing: ignore_missing.unwrap_or(false),
        settings,
    })
    .await?;

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
    let result = collection_service::update_collection(pool.inner(), input).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_collection(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, OperationLock>,
    id: String,
) -> Result<(), AppError> {
    let _guard = op_lock
        .inner()
        .acquire()
        .await
        .map_err(AppError::Internal)?;
    collection_service::delete_collection(pool.inner(), &id).await?;
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

    let result = collection_service::get_collection_preview(
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
    is_safe: bool,
) -> Result<ApplyPreview, AppError> {
    let settings = config.get_settings();
    let mods_path = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .map(|g| g.mod_path.to_string_lossy().to_string());

    let result = collection_service::preview_apply(
        pool.inner(),
        &game_id,
        &collection_id,
        is_safe,
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
    let tasks = crate::repo::task_repo::get_all_pending_tasks_global(pool.inner()).await?;
    Ok(tasks)
}

#[tauri::command]
#[specta::specta]
pub async fn check_boot_security(
    pool: State<'_, SqlitePool>,
    is_safe_mode: bool,
) -> Result<bool, AppError> {
    let result = pin_service::check_boot_security(pool.inner(), is_safe_mode).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_recovery_task(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    task_id: String,
    action: crate::domain::task::RecoveryAction,
) -> Result<(), AppError> {
    log::info!(
        "Resolving recovery task {} with action {:?}",
        task_id,
        action
    );

    let task = crate::repo::task_repo::get_task_by_id(pool.inner(), &task_id)
        .await?
        .ok_or_else(|| AppError::Validation(format!("Task {} not found", task_id)))?;

    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|g| g.id == task.game_id)
        .ok_or_else(|| AppError::Validation(format!("Game {} not found", task.game_id)))?;

    match action {
        crate::domain::task::RecoveryAction::Retry => {
            match task.task_type.as_str() {
                "apply_collection" => {
                    let Some(collection_id) = task.target_id.as_deref() else {
                        return Err(AppError::Validation(
                            "Missing target collection ID".to_string(),
                        ));
                    };
                    let collection =
                        crate::repo::collection_repo::get_by_id(pool.inner(), collection_id)
                            .await?
                            .ok_or_else(|| {
                                AppError::Validation(format!(
                                    "Collection {} not found",
                                    collection_id
                                ))
                            })?;

                    crate::services::collection_service::apply_collection(
                        crate::services::collection_service::ApplyCollectionRequest {
                            pool: pool.inner(),
                            game_id: &task.game_id,
                            collection_id,
                            is_safe: collection.is_safe,
                            mods_path: game.mod_path.clone(),
                            suppressor: watcher_state.suppressor.clone(),
                            ignore_missing: true,
                            settings,
                        },
                    )
                    .await?;
                }
                "switch_corridor" => {
                    let Some(target_safe_str) = task.target_id.as_deref() else {
                        return Err(AppError::Validation(
                            "Missing target corridor state".to_string(),
                        ));
                    };
                    let target_safe = target_safe_str == "true";
                    crate::services::corridor_service::switch_corridor(
                        pool.inner(),
                        &task.game_id,
                        target_safe,
                        game.mod_path.clone(),
                        watcher_state.suppressor.clone(),
                        &watcher_state,
                        settings,
                    )
                    .await?;
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported task type for retry: {}",
                        task.task_type
                    )));
                }
            }

            crate::repo::task_repo::update_status(
                pool.inner(),
                &task_id,
                crate::domain::task::TaskStatus::Completed,
            )
            .await?;
            Ok(())
        }
        crate::domain::task::RecoveryAction::Rollback => {
            match task.task_type.as_str() {
                "switch_corridor" => {
                    let Some(target_safe_str) = task.target_id.as_deref() else {
                        return Err(AppError::Validation(
                            "Missing target corridor state".to_string(),
                        ));
                    };
                    let target_safe = target_safe_str == "true";
                    crate::services::corridor_service::switch_corridor(
                        pool.inner(),
                        &task.game_id,
                        !target_safe,
                        game.mod_path.clone(),
                        watcher_state.suppressor.clone(),
                        &watcher_state,
                        settings,
                    )
                    .await?;
                }
                "apply_collection" => {
                    let Some(collection_id) = task.target_id.as_deref() else {
                        return Err(AppError::Validation(
                            "Missing target collection ID".to_string(),
                        ));
                    };
                    let applied_collection =
                        crate::repo::collection_repo::get_by_id(pool.inner(), collection_id)
                            .await?
                            .ok_or_else(|| {
                                AppError::Validation(format!(
                                    "Collection {} not found",
                                    collection_id
                                ))
                            })?;
                    let corridor_state = crate::repo::corridor_repo::get(
                        pool.inner(),
                        &task.game_id,
                        applied_collection.is_safe,
                    )
                    .await?;

                    let rollback_collection_id = if let Some(existing_id) = corridor_state
                        .as_ref()
                        .and_then(|state| state.undo_collection_id.as_deref())
                        .or_else(|| {
                            corridor_state
                                .as_ref()
                                .and_then(|state| state.active_collection_id.as_deref())
                                .filter(|candidate| *candidate != collection_id)
                        }) {
                        Some(existing_id.to_string())
                    } else {
                        crate::services::corridor_service::resolve_restore_collection(
                            pool.inner(),
                            &task.game_id,
                            applied_collection.is_safe,
                        )
                        .await
                        .ok()
                        .flatten()
                        .and_then(|(collection, _)| {
                            if collection.id == collection_id {
                                None
                            } else {
                                Some(collection.id)
                            }
                        })
                    };

                    let Some(rollback_collection_id) = rollback_collection_id else {
                        return Err(AppError::Validation(
                            "No rollback collection is available for this corridor".to_string(),
                        ));
                    };

                    crate::services::collection_service::apply_collection(
                        crate::services::collection_service::ApplyCollectionRequest {
                            pool: pool.inner(),
                            game_id: &task.game_id,
                            collection_id: &rollback_collection_id,
                            is_safe: applied_collection.is_safe,
                            mods_path: game.mod_path.clone(),
                            suppressor: watcher_state.suppressor.clone(),
                            ignore_missing: true,
                            settings,
                        },
                    )
                    .await?;
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "Unsupported task type for rollback: {}",
                        task.task_type
                    )));
                }
            }

            crate::repo::task_repo::update_status(
                pool.inner(),
                &task_id,
                crate::domain::task::TaskStatus::Completed,
            )
            .await?;
            Ok(())
        }
        crate::domain::task::RecoveryAction::Ignore => {
            crate::repo::task_repo::update_status(
                pool.inner(),
                &task_id,
                crate::domain::task::TaskStatus::Failed,
            )
            .await?;
            Ok(())
        }
    }
}

// ============================================================================
// PIN Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn has_pin(pool: State<'_, SqlitePool>) -> Result<bool, AppError> {
    let result = pin_service::has_pin(pool.inner()).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn set_pin(
    pool: State<'_, SqlitePool>,
    pin: String,
    recovery_code: Option<String>,
) -> Result<(), AppError> {
    pin_service::set_pin(pool.inner(), &pin, recovery_code.as_deref()).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn verify_pin(pool: State<'_, SqlitePool>, pin: String) -> Result<bool, AppError> {
    let result = pin_service::verify_pin(pool.inner(), &pin).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_pin(pool: State<'_, SqlitePool>) -> Result<(), AppError> {
    pin_service::clear_pin(pool.inner()).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_pin_status(pool: State<'_, SqlitePool>) -> Result<PinStatus, AppError> {
    let result = pin_service::get_status(pool.inner()).await?;
    Ok(result)
}
