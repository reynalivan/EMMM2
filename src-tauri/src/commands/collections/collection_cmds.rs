use crate::services::collections::{
    apply_collection as apply_collection_service, create_collection as create_collection_service,
    delete_collection as delete_collection_service,
    get_collection_preview as get_collection_preview_service,
    list_collections as list_collections_service, undo_collection as undo_collection_service,
    update_collection as update_collection_service, ApplyCollectionResult, Collection,
    CollectionDetails, CollectionPreviewMod, CreateCollectionInput, UpdateCollectionInput,
};
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn list_collections(
    pool: State<'_, SqlitePool>,
    config: State<'_, ConfigService>,
    game_id: String,
) -> Result<Vec<Collection>, String> {
    let safe_mode_enabled = config.get_settings().safe_mode.enabled;
    list_collections_service(pool.inner(), &game_id, safe_mode_enabled).await
}

#[tauri::command]
pub async fn create_collection(
    pool: State<'_, SqlitePool>,
    input: CreateCollectionInput,
) -> Result<CollectionDetails, String> {
    create_collection_service(pool.inner(), input).await
}

#[tauri::command]
pub async fn update_collection(
    pool: State<'_, SqlitePool>,
    input: UpdateCollectionInput,
) -> Result<CollectionDetails, String> {
    update_collection_service(pool.inner(), input).await
}

#[tauri::command]
pub async fn delete_collection(
    pool: State<'_, SqlitePool>,
    id: String,
    game_id: String,
) -> Result<(), String> {
    delete_collection_service(pool.inner(), &id, &game_id).await
}

#[tauri::command]
pub async fn apply_collection(
    pool: State<'_, SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    config: State<'_, ConfigService>,
    collection_id: String,
    game_id: String,
) -> Result<ApplyCollectionResult, String> {
    let _lock = op_lock.acquire().await?;
    let safe_mode_enabled = config.get_settings().safe_mode.enabled;
    apply_collection_service(
        pool.inner(),
        &watcher_state,
        &collection_id,
        &game_id,
        safe_mode_enabled,
    )
    .await
}

#[tauri::command]
pub async fn get_collection_preview(
    pool: State<'_, SqlitePool>,
    collection_id: String,
    game_id: String,
) -> Result<Vec<CollectionPreviewMod>, String> {
    get_collection_preview_service(pool.inner(), &collection_id, &game_id).await
}

#[tauri::command]
pub async fn undo_collection(
    pool: State<'_, SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    config: State<'_, ConfigService>,
    game_id: String,
) -> Result<ApplyCollectionResult, String> {
    let _lock = op_lock.acquire().await?;
    let safe_mode_enabled = config.get_settings().safe_mode.enabled;
    undo_collection_service(pool.inner(), &watcher_state, &game_id, safe_mode_enabled).await
}
#[cfg(test)]
#[path = "tests/collection_cmds_tests.rs"]
mod tests;
