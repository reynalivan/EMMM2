use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::dedup::resolver::{
    ResolutionProgress, ResolutionRequest, ResolutionSummary,
};
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn dup_resolve_batch(
    app: AppHandle,
    requests: Vec<ResolutionRequest>,
    game_id: String,
    watcher_state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    config: State<'_, ConfigService>,
    db: State<'_, sqlx::SqlitePool>,
) -> Result<ResolutionSummary, AppError> {
    // Every request path must stay inside this game's mods root before any
    // trash move or hardlink touches the filesystem. Batched so the root is
    // canonicalized once, not twice per request.
    let all_folders: Vec<String> = requests
        .iter()
        .flat_map(|request| [request.folder_a.clone(), request.folder_b.clone()])
        .collect();
    crate::services::fs_utils::guard::validate_paths(&config, &game_id, &all_folders)?;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        AppError::Internal(format!("Failed to get app data directory: {error}"))
    })?;
    let trash_dir = crate::services::mods::trash::trash_dir_under(&app_data_dir);

    let op_guard = op_lock.acquire().await?;
    crate::services::scanner::dedup::resolver::resolve_batch(
        requests,
        game_id,
        db.inner(),
        &op_guard,
        &watcher_state.suppressor,
        &trash_dir,
        |progress: ResolutionProgress| {
            let _ = app.emit("dup-resolve-progress", &progress);
        },
    )
    .await
}
