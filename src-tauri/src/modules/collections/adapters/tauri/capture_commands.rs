use super::acquire_current_snapshot_guard;
use crate::modules::collections::application::collection;
use crate::modules::collections::domain::collection::{
    CollectionSummary, CreateCollectionInput, CreateCollectionMode, UpdateCollectionInput,
};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};
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
