use tauri::{Manager, State};

use crate::types::errors::CommandResult;

use crate::database::object_repo::{
    CategoryCount, CreateObjectInput, GetObjectsResult, ObjectFilter, UpdateObjectInput,
};

#[tauri::command]
pub async fn get_objects_cmd(
    filter: ObjectFilter,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<GetObjectsResult> {
    get_objects_cmd_inner(filter, &pool).await
}

#[tauri::command]
pub async fn sync_objects_cmd(
    game_id: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<()> {
    crate::services::scanner::object_sync::sync_objects_for_game(&pool, &game_id).await
}

pub async fn get_objects_cmd_inner(
    filter: ObjectFilter,
    pool: &sqlx::SqlitePool,
) -> CommandResult<GetObjectsResult> {
    let objects =
        crate::services::objects::query::get_filtered_objects_with_conflict_check(pool, &filter)
            .await
            .map_err(|e| crate::types::errors::CommandError::Database(e))?;

    Ok(objects)
}

#[tauri::command]
pub async fn get_category_counts_cmd(
    game_id: String,
    safe_mode: bool,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<Vec<CategoryCount>> {
    let counts =
        crate::services::objects::query::get_category_counts_service(&pool, &game_id, safe_mode)
            .await
            .map_err(|e| crate::types::errors::CommandError::App(e))?;

    Ok(counts)
}

#[tauri::command]
pub async fn create_object_cmd(
    input: CreateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
    app: tauri::AppHandle,
) -> CommandResult<String> {
    create_object_cmd_inner(input, &pool, Some(&app)).await
}

pub async fn create_object_cmd_inner(
    input: CreateObjectInput,
    pool: &sqlx::SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
) -> CommandResult<String> {
    crate::services::objects::mutate::create_object_cmd_inner(pool, app_handle, input).await
}

#[tauri::command]
pub async fn update_object_cmd(
    id: String,
    updates: UpdateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<()> {
    update_object_cmd_inner(id, &updates, &pool).await
}

pub async fn update_object_cmd_inner(
    id: String,
    updates: &UpdateObjectInput,
    pool: &sqlx::SqlitePool,
) -> CommandResult<()> {
    crate::services::objects::mutate::update_object(pool, &id, updates).await
}

#[tauri::command]
pub async fn delete_object_cmd(
    id: String,
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> CommandResult<()> {
    let trash_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| {
            crate::types::errors::CommandError::Io(format!("Failed to get app data dir: {}", e))
        })?
        .join("trash");
    crate::services::objects::mutate::delete_object(&pool, &id, &trash_dir, &state, &op_lock).await
}

/// Garbage-collect objects whose folders no longer exist on disk.
/// Called at sync points (game switch, manual sync) — NOT on every ObjectList render.
#[tauri::command]
pub async fn gc_lost_objects_cmd(
    game_id: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<Vec<String>> {
    let lost = crate::services::objects::query::gc_lost_objects(&pool, &game_id)
        .await
        .map_err(|e| crate::types::errors::CommandError::App(e))?;
    Ok(lost)
}

#[cfg(test)]
#[path = "tests/object_cmds_tests.rs"]
mod tests;
