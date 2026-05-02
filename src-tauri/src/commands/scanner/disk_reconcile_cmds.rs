use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn reconcile_disk_state_cmd(
    app: tauri::AppHandle,
    game_id: String,
    reason: crate::services::disk_reconcile::types::DiskReconcileReason,
    changed_paths: Option<Vec<String>>,
    force_full: Option<bool>,
    pool: State<'_, sqlx::SqlitePool>,
    config: State<'_, crate::services::config::ConfigService>,
    watcher: State<'_, crate::services::scanner::watcher::WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::services::disk_reconcile::orchestrator::DiskReconcileState,
    >,
) -> Result<crate::services::disk_reconcile::types::DiskReconcileResult, String> {
    crate::services::disk_reconcile::orchestrator::reconcile_disk_state(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        watcher.suppressor.clone(),
        game_id,
        reason,
        changed_paths.unwrap_or_default(),
        force_full.unwrap_or(false),
    )
    .await
}
