use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::trash;
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, Manager, State};

#[specta::specta]
#[tauri::command]
pub async fn delete_mod(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    game_id: Option<String>,
) -> Result<trash::DeleteModResult, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("Failed to get app data dir: {}", e)))?;
    let trash_dir = app_data_dir.join("trash");

    let result =
        trash::delete_mod_service(&config, &pool, &state, &op_lock, trash_dir, path, game_id).await;

    result
}

#[specta::specta]
#[tauri::command]
pub async fn restore_mod(
    app: AppHandle,
    trash_id: String,
    game_id: Option<String>,
) -> Result<String, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("Failed to get app data dir: {e}")))?;

    let result =
        trash::restore_from_trash(&trash_id, &app_data_dir.join("trash"), game_id.as_ref())?;

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn list_trash(app: AppHandle) -> Result<Vec<trash::TrashMetadata>, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("Failed to get app data dir: {e}")))?;
    trash::list_trash(&app_data_dir.join("trash"))
}

#[specta::specta]
#[tauri::command]
pub async fn empty_trash(app: AppHandle) -> Result<u64, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("Failed to get app data dir: {e}")))?;
    trash::empty_trash(&app_data_dir.join("trash"))
}

#[cfg(test)]
#[path = "tests/trash_cmds_tests.rs"]
mod tests;
