use crate::domain::errors::AppError;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::core::types::{CollisionInfo, CollisionResolution};
use crate::services::scanner::watcher::WatcherState;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn resolve_folder_collision(
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    collision: CollisionInfo,
    resolution: CollisionResolution,
) -> Result<String, AppError> {
    crate::services::mods::collision_resolver::resolve_collision_service(
        &pool, &watcher, &op_lock, &game_id, collision, resolution,
    )
    .await
}
