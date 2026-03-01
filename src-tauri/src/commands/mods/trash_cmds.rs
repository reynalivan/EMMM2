use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::trash;
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn delete_mod(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    game_id: Option<String>,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let trash_dir = app_data_dir.join("trash");

    trash::delete_mod_service(&pool, &state, &op_lock, trash_dir, path, game_id).await
}

#[tauri::command]
pub async fn restore_mod(
    app: AppHandle,
    trash_id: String,
    game_id: Option<String>,
) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    trash::restore_from_trash(&trash_id, &app_data_dir.join("trash"), game_id.as_ref())
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

#[cfg(test)]
#[path = "tests/trash_cmds_tests.rs"]
mod tests;
