use tauri::{Manager, State};

use crate::domain::errors::AppError;

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
) -> Result<GetObjectsResult, AppError> {
    get_objects_cmd_inner(filter, &pool).await
}

pub async fn get_objects_cmd_inner(
    filter: ObjectFilter,
    pool: &sqlx::SqlitePool,
) -> Result<GetObjectsResult, AppError> {
    let objects =
        crate::services::objects::query::get_filtered_objects_with_conflict_check(pool, &filter)
            .await
            .map_err(AppError::Db)?;

    Ok(objects)
}

#[tauri::command]
#[specta::specta]
pub async fn get_category_counts_cmd(
    game_id: String,
    safe_mode: bool,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<CategoryCount>, AppError> {
    let counts =
        crate::services::objects::query::get_category_counts_service(&pool, &game_id, safe_mode)
            .await
            .map_err(|e| AppError::Validation(e.to_string()))?;

    Ok(counts)
}

#[tauri::command]
#[specta::specta]
pub async fn create_object_cmd(
    input: CreateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
    app: tauri::AppHandle,
    watcher: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<String, AppError> {
    let _lock = op_lock.acquire().await?;
    let _guard = crate::services::scanner::watcher::SuppressionGuard::new(&watcher.suppressor);
    crate::services::objects::mutate::create_object_cmd_inner(&pool, Some(&app), input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_object_cmd(
    id: String,
    updates: UpdateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(), AppError> {
    crate::services::objects::mutate::update_object(&pool, &id, &updates).await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_object_match_cmd(
    input: ApplyObjectMatchInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(), AppError> {
    apply_object_match_cmd_inner(&input, &pool).await
}

pub async fn apply_object_match_cmd_inner(
    input: &ApplyObjectMatchInput,
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    crate::services::objects::matching::apply_object_match(
        pool,
        &input.game_id,
        input.object_id.as_deref(),
        input.folder_path.as_deref(),
        crate::services::objects::matching::ObjectMatchFields {
            entry_key: input.matched_entry_key.as_deref(),
            alias_name: input.matched_alias_name.as_deref(),
            confidence: input.matched_confidence,
            reason: input.matched_reason.as_deref(),
            source: input.matched_source.as_deref(),
        },
    )
    .await
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
) -> Result<(), AppError> {
    let trash_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("Failed to get app data dir: {}", e)))?
        .join("trash");
    crate::services::objects::mutate::delete_object(&pool, &id, force, &trash_dir, &state, &op_lock)
        .await
}

#[cfg(test)]
#[path = "tests/object_cmds_tests.rs"]
mod tests;
