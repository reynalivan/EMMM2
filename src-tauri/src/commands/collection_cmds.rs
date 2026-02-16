use crate::services::collections::{
    apply_collection as apply_collection_service, create_collection as create_collection_service,
    delete_collection as delete_collection_service, export_collection as export_collection_service,
    import_collection as import_collection_service, list_collections as list_collections_service,
    undo_collection_apply as undo_collection_apply_service,
    update_collection as update_collection_service, ApplyCollectionResult, Collection,
    CollectionDetails, CollectionsUndoState, CreateCollectionInput, ExportCollectionPayload,
    ImportCollectionResult, UndoCollectionResult, UpdateCollectionInput,
};
use crate::services::config::ConfigService;
use crate::services::operation_lock::OperationLock;
use crate::services::watcher::WatcherState;
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
    undo_state: State<'_, CollectionsUndoState>,
    config: State<'_, ConfigService>,
    collection_id: String,
    game_id: String,
) -> Result<ApplyCollectionResult, String> {
    let _lock = op_lock.acquire().await?;
    let safe_mode_enabled = config.get_settings().safe_mode.enabled;
    apply_collection_service(
        pool.inner(),
        &watcher_state,
        &undo_state,
        &collection_id,
        &game_id,
        safe_mode_enabled,
    )
    .await
}

#[tauri::command]
pub async fn undo_collection_apply(
    pool: State<'_, SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    undo_state: State<'_, CollectionsUndoState>,
    game_id: String,
) -> Result<UndoCollectionResult, String> {
    let _lock = op_lock.acquire().await?;
    undo_collection_apply_service(pool.inner(), &watcher_state, &undo_state, &game_id).await
}

#[tauri::command]
pub async fn export_collection(
    pool: State<'_, SqlitePool>,
    collection_id: String,
    game_id: String,
) -> Result<ExportCollectionPayload, String> {
    export_collection_service(pool.inner(), &collection_id, &game_id).await
}

#[tauri::command]
pub async fn import_collection(
    pool: State<'_, SqlitePool>,
    game_id: String,
    payload: ExportCollectionPayload,
) -> Result<ImportCollectionResult, String> {
    import_collection_service(pool.inner(), &game_id, payload).await
}
