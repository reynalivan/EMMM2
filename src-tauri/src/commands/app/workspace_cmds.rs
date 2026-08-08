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
    mut input: WorkspaceViewModelInput,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
) -> Result<WorkspaceViewModel, AppError> {
    // The Safe Mode corridor is a privacy gate, so it is read server-side and
    // overrides whatever the client sent. The explorer/preview filters compare
    // it with `==`, so an unchecked `safe_mode: false` would return exclusively
    // unsafe folders — their names, paths, previews and INI summaries.
    input.filter.safe_mode = config.current_corridor().is_safe();

    crate::services::workspace_service::get_workspace_view_model(pool.inner(), input).await
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
    let op_guard = op_lock.acquire().await?;
    crate::services::workspace_switch_service::execute_switch(
        &app,
        input,
        config.inner(),
        pool.inner(),
        watcher_state.inner(),
        &op_guard,
    )
    .await
}
