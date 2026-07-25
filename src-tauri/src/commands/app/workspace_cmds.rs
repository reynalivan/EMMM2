use tauri::State;

use crate::domain::errors::AppError;
use crate::domain::workspace::{
    WorkspaceSwitchInput, WorkspaceSwitchResult, WorkspaceViewModel, WorkspaceViewModelInput,
};
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_view_model(
    input: WorkspaceViewModelInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<WorkspaceViewModel, AppError> {
    crate::services::workspace_service::get_workspace_view_model(pool.inner(), input)
        .await
        .map_err(AppError::Internal)
}

#[tauri::command]
#[specta::specta]
pub async fn execute_workspace_switch(
    app: tauri::AppHandle,
    input: WorkspaceSwitchInput,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
) -> Result<WorkspaceSwitchResult, AppError> {
    crate::services::workspace_switch_service::execute_switch(
        &app,
        input,
        config.inner(),
        pool.inner(),
        watcher_state.inner(),
        op_lock.inner(),
    )
    .await
}
