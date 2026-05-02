use tauri::{Manager, State};

use crate::types::errors::CommandResult;

use crate::repo::object_repo::{
    CategoryCount, CreateObjectInput, GetObjectsResult, ObjectFilter, UpdateObjectInput,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ApplyObjectMatchInput {
    pub game_id: String,
    pub object_id: Option<String>,
    pub folder_path: Option<String>,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
    pub matched_confidence: Option<f64>,
    pub matched_reason: Option<String>,
    pub matched_source: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_objects_cmd(
    filter: ObjectFilter,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<GetObjectsResult> {
    get_objects_cmd_inner(filter, &pool).await
}

pub async fn get_objects_cmd_inner(
    filter: ObjectFilter,
    pool: &sqlx::SqlitePool,
) -> CommandResult<GetObjectsResult> {
    let objects =
        crate::services::objects::query::get_filtered_objects_with_conflict_check(pool, &filter)
            .await
            .map_err(crate::types::errors::CommandError::Database)?;

    Ok(objects)
}

#[tauri::command]
#[specta::specta]
pub async fn get_category_counts_cmd(
    game_id: String,
    safe_mode: bool,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<Vec<CategoryCount>> {
    let counts =
        crate::services::objects::query::get_category_counts_service(&pool, &game_id, safe_mode)
            .await
            .map_err(|e| crate::types::errors::CommandError::App(e.to_string()))?;

    Ok(counts)
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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
#[specta::specta]
pub async fn apply_object_match_cmd(
    input: ApplyObjectMatchInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<()> {
    apply_object_match_cmd_inner(&input, &pool).await
}

pub async fn apply_object_match_cmd_inner(
    input: &ApplyObjectMatchInput,
    pool: &sqlx::SqlitePool,
) -> CommandResult<()> {
    let target_object_id = match input.object_id.as_deref() {
        Some(object_id) => object_id.to_string(),
        None => {
            let folder_path = input.folder_path.as_deref().ok_or_else(|| {
                crate::types::errors::CommandError::App(
                    "apply_object_match_cmd requires object_id or folder_path".to_string(),
                )
            })?;

            crate::repo::mod_repo::get_object_id_by_folder_and_game(
                pool,
                folder_path,
                &input.game_id,
            )
            .await
            .map_err(|error| crate::types::errors::CommandError::Database(error.to_string()))?
            .ok_or_else(|| {
                crate::types::errors::CommandError::NotFound(format!(
                    "No physical object found for folder '{}'",
                    folder_path
                ))
            })?
        }
    };

    crate::repo::object_repo::apply_canonical_match(
        pool,
        &target_object_id,
        input.matched_entry_key.as_deref(),
        input.matched_alias_name.as_deref(),
        input.matched_confidence,
        input.matched_reason.as_deref(),
        Some(input.matched_source.as_deref().unwrap_or("manual_match")),
    )
    .await
    .map_err(|error| crate::types::errors::CommandError::Database(error.to_string()))?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_object_cmd(
    id: String,
    force: bool,
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
    crate::services::objects::mutate::delete_object(&pool, &id, force, &trash_dir, &state, &op_lock)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn pin_object_cmd(
    id: String,
    pin: bool,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<()> {
    crate::services::objects::mutate::toggle_pin_object(&pool, &id, pin)
        .await
        .map_err(|e| crate::types::errors::CommandError::App(e.to_string()))
}

#[cfg(test)]
#[path = "tests/object_cmds_tests.rs"]
mod tests;
