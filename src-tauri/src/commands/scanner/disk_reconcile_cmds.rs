use tauri::State;

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn reconcile_disk_state_cmd(
    _app: tauri::AppHandle,
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
        crate::services::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: pool.inner(),
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
        },
        crate::services::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
            game_id,
            reason,
            changed_paths.unwrap_or_default(),
            force_full.unwrap_or(false),
        ),
    )
    .await
}
