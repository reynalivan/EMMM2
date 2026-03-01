use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::bulk;
use crate::services::mods::info_json;
use crate::services::scanner::watcher::WatcherState;
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn bulk_toggle_mods(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, String> {
    let _lock = op_lock.acquire().await?;
    bulk::bulk_toggle(&app, &pool, &state, paths, enable).await
}

pub async fn bulk_toggle_mods_inner(
    state: &WatcherState,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, String> {
    bulk::bulk_toggle_inner(state, paths, enable).await
}

#[tauri::command]
pub async fn bulk_delete_mods(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<bulk::BulkResult, String> {
    let _lock = op_lock.acquire().await?;
    bulk::bulk_delete(&app, &pool, &state, paths, game_id).await
}

pub async fn bulk_delete_mods_inner(
    state: &WatcherState,
    trash_dir: &Path,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<bulk::BulkResult, String> {
    bulk::bulk_delete_inner(state, trash_dir, paths, game_id).await
}

#[tauri::command]
pub async fn bulk_update_info(
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, String> {
    bulk::bulk_update_info(paths, update).await
}

#[tauri::command]
pub async fn bulk_toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, String> {
    bulk::bulk_toggle_favorite(&pool, game_id, folder_paths, favorite).await
}

#[tauri::command]
pub async fn bulk_pin_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, String> {
    bulk::bulk_pin(&pool, game_id, folder_paths, pin).await
}

#[cfg(test)]
#[path = "tests/mod_bulk_cmds_tests.rs"]
mod tests;
