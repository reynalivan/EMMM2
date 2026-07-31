use sqlx::SqlitePool;
use tauri::State;

use crate::domain::collection::{
    ApplyPreview, ApplyProgressSnapshot, ApplyResult, CollectionPreview, CollectionSummary,
    CreateCollectionInput, CreateCollectionMode, UpdateCollectionInput,
};
use crate::domain::corridor::CorridorSnapshot;
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
    is_safe: Option<bool>,
) -> Result<CorridorSnapshot, AppError> {
    let snapshot =
        corridor_service::get_corridor_state(pool.inner(), &game_id, is_safe.unwrap_or(true))
            .await?;
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

// ============================================================================
// Collection Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub async fn list_collections(
    pool: State<'_, SqlitePool>,
    game_id: String,
    is_safe: Option<bool>,
) -> Result<Vec<CollectionSummary>, AppError> {
    let result =
        collection_service::list_collections(pool.inner(), &game_id, is_safe.unwrap_or(true), None)
            .await?;
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
    let _guard = op_lock.inner().acquire().await?;
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
pub async fn replace_collection_with_current_state(
    pool: State<'_, SqlitePool>,
    game_id: String,
    collection_id: String,
) -> Result<CollectionSummary, AppError> {
    let result = collection_service::replace_collection_with_current_state(
        pool.inner(),
        &game_id,
        &collection_id,
    )
    .await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_collection(
    pool: State<'_, SqlitePool>,
    op_lock: State<'_, OperationLock>,
    id: String,
) -> Result<(), AppError> {
    let _guard = op_lock.inner().acquire().await?;
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
    is_safe: Option<bool>,
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
        is_safe.unwrap_or(true),
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
    crate::repo::task_repo::get_all_pending_tasks_global(pool.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_recovery_task(
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, OperationLock>,
    task_id: String,
    action: crate::domain::task::RecoveryAction,
) -> Result<(), AppError> {
    // Resuming an interrupted apply mutates the filesystem, so it must be
    // mutually excluded from concurrent runtime ops just like a normal apply.
    let _guard = op_lock.inner().acquire().await?;

    crate::services::recovery_service::resolve_recovery_task(
        pool.inner(),
        config.inner(),
        watcher_state.inner(),
        &task_id,
        action,
    )
    .await
}

// ============================================================================
// PIN Commands
// ============================================================================

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
pub async fn get_pin_status(pool: State<'_, SqlitePool>) -> Result<PinStatus, AppError> {
    let result = pin_service::get_status(pool.inner()).await?;
    Ok(result)
}
