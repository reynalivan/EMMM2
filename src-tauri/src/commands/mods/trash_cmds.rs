use crate::services::core::operation_lock::OperationLock;
use crate::services::mod_files::trash;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn delete_mod(
    app: AppHandle,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    game_id: Option<String>,
) -> Result<(), String> {
    let _lock = op_lock.acquire().await?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let trash_dir = app_data_dir.join("trash");

    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir).map_err(|e| format!("Failed to create trash dir: {}", e))?;
    }

    delete_mod_inner(&state, &trash_dir, path, game_id).await
}

pub async fn delete_mod_inner(
    state: &WatcherState,
    trash_dir: &Path,
    path: String,
    game_id: Option<String>,
) -> Result<(), String> {
    let path_obj = Path::new(&path);
    let _guard = SuppressionGuard::new(&state.suppressor);
    trash::move_to_trash(path_obj, trash_dir, game_id).map(|_| ())
}

#[tauri::command]
pub async fn restore_mod(app: AppHandle, trash_id: String) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    trash::restore_from_trash(&trash_id, &app_data_dir.join("trash"))
}

#[tauri::command]
pub async fn list_trash(app: AppHandle) -> Result<Vec<trash::TrashMetadata>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    trash::list_trash(&app_data_dir.join("trash"))
}

#[tauri::command]
pub async fn empty_trash(app: AppHandle) -> Result<u64, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    trash::empty_trash(&app_data_dir.join("trash"))
}
