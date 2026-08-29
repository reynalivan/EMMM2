use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::dedup::resolver::{
    ResolutionProgress, ResolutionRequest, ResolutionSummary,
};
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, Emitter, State};

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
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        db.inner(),
        &game_id,
        Some(&all_folders),
    )
    .await?;

    let op_guard = op_lock.acquire().await?;
    let result = crate::services::scanner::dedup::resolver::resolve_batch(
        requests,
        game_id.clone(),
        db.inner(),
        &op_guard,
        &watcher_state.suppressor,
        |progress: ResolutionProgress| {
            let _ = app.emit("dup-resolve-progress", &progress);
        },
    )
    .await?;
    drop(op_guard);
    crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile(
        &app,
        db.inner(),
        &game_id,
    )
    .await?;
    Ok(result)
}
