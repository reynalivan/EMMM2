//! Shared convergence hook for internal mutations.
//!
//! Runs a scoped `InternalMutation` Disk Reconcile and emits the result to
//! the frontend. Called after internal FS mutations so the runtime projection
//! converges with disk reality even if a manual DB sync step missed a case
//! or watcher events were dropped during suppression.

use crate::domain::errors::AppError;
use tauri::{Emitter, Manager};

use crate::services::disk_reconcile::orchestrator::{
    reconcile_disk_state, DiskReconcileContext, DiskReconcileRequest, DiskReconcileState,
};
use crate::services::disk_reconcile::types::DiskReconcileReason;

/// Scoped `InternalMutation` reconcile without the frontend event. For flows
/// whose command RESULT already drives the frontend refresh (workspace
/// switch): emitting the event too would trigger a second full
/// invalidation+refetch round per toggle.
pub async fn run_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
) -> Result<crate::services::disk_reconcile::types::DiskReconcileResult, AppError> {
    let config = app
        .try_state::<crate::services::config::ConfigService>()
        .ok_or_else(|| {
            AppError::Internal("ConfigService state missing for disk reconcile".to_string())
        })?;
    let watcher = app
        .try_state::<crate::services::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState missing for disk reconcile".to_string()))?;
    let disk_reconcile_state = app.try_state::<DiskReconcileState>().ok_or_else(|| {
        AppError::Internal("DiskReconcileState missing for disk reconcile".to_string())
    })?;

    reconcile_disk_state(
        DiskReconcileContext {
            pool,
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: watcher.suppressor.clone(),
        },
        DiskReconcileRequest::manual(
            game_id.to_string(),
            DiskReconcileReason::InternalMutation,
            changed_paths,
            false,
        ),
    )
    .await
}

pub async fn emit_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    changed_paths: Vec<String>,
) -> Result<(), AppError> {
    let result = run_internal_disk_reconcile(app, pool, game_id, changed_paths).await?;
    Ok(app.emit("disk_reconcile:result", result)?)
}
