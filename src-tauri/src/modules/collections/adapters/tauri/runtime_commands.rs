use crate::modules::collections::application::collection;
use crate::modules::collections::application::runtime as collection_runtime;
use crate::modules::collections::domain::collection::{ApplyProgressSnapshot, CollectionSummary};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::workspace::domain::runtime_state::{
    CollectionRuntimeDescriptor, CollectionRuntimeSnapshot,
};
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::State;
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
pub async fn app_startup_check(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<crate::modules::workspace::domain::task::PipelineTask>, AppError> {
    crate::modules::workspace::application::recovery::get_startup_recovery_tasks(pool.inner()).await
}
