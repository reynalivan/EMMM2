use crate::services::core::operation_lock::OperationLock;
use crate::services::scanner::dedup::resolver::{
    ResolutionProgress, ResolutionRequest, ResolutionSummary,
};
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn dup_resolve_batch(
    app: AppHandle,
    requests: Vec<ResolutionRequest>,
    game_id: String,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<ResolutionSummary, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data directory: {error}"))?;
    let trash_dir = app_data_dir.join("trash");

    crate::services::scanner::dedup::resolver::resolve_batch(
        requests,
        game_id,
        db.inner(),
        op_lock.inner(),
        &watcher_state.suppressor,
        &trash_dir,
        |progress: ResolutionProgress| {
            let _ = app.emit("dup-resolve-progress", &progress);
        },
    )
    .await
}
