//! Watcher lifecycle management.
//!
//! The watcher is now a pure trigger source:
//! - collect filesystem events
//! - debounce into batches
//! - delegate Disk Reconcile to `disk_reconcile`
//! - emit typed payloads back to the frontend

use crate::common::sync::lock;
use crate::domain::errors::ScannerError;
use crate::services::scanner::watcher::{
    ModWatchEvent, WatchEventPayload, WatcherState, WatcherSuppressor,
};
use std::sync::Arc;
use tauri::{Emitter, Manager};

fn emit_event(app: &tauri::AppHandle, payload: WatchEventPayload) {
    let _ = app.emit("mod_watch:event", payload);
}

pub fn start_watcher(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
) -> Result<(), ScannerError> {
    let path_obj = std::path::Path::new(&path);

    // A fresh watcher session must not inherit stale frontend suppression
    // (e.g. webview reloaded mid-operation).
    state.suppressor.reset_manual();

    log::info!("Starting watcher on: {}", path);

    let (watcher, rx) =
        crate::services::scanner::watcher::watch_mod_directory(path_obj, state.suppressor.clone())?;

    {
        // Single lock: stop the old watcher and install the new one atomically
        // so overlapping start/stop commands cannot interleave.
        let mut active_watcher = lock(&state.watcher);
        if active_watcher.is_some() {
            log::info!("Stopping existing watcher");
        }
        *active_watcher = Some(watcher);
    }

    let app_handle = app.clone();
    let db_pool = pool;
    let mods_path_root = path;
    let suppressor = state.suppressor.clone();

    tokio::spawn(async move {
        process_event_loop(rx, app_handle, db_pool, game_id, mods_path_root, suppressor).await;
    });

    Ok(())
}

async fn process_event_loop(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<ModWatchEvent>,
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
    mods_path_root: String,
    suppressor: Arc<WatcherSuppressor>,
) {
    loop {
        // The debouncer already batches (one callback per debounce window and
        // it sends its whole batch synchronously), so a recv + drain
        // reassembles it without extra timers here.
        let mut batch = Vec::new();
        let Some(first_event) = rx.recv().await else {
            break;
        };
        batch.push(first_event);
        while let Ok(event) = rx.try_recv() {
            batch.push(event);
        }

        log::debug!("Watcher flushing batched events: {}", batch.len());

        // notify emits errors on Windows ReadDirectoryChangesW buffer overflow
        // during mass renames — events were LOST, so a scoped reconcile of the
        // known paths is not enough. Fall back to a full pass.
        let events_lost = batch
            .iter()
            .any(|event| matches!(event, ModWatchEvent::Error(_)));
        for event in &batch {
            if let ModWatchEvent::Error(error) = event {
                log::warn!("Watcher error for {}: {}", mods_path_root, error);
            }
        }

        let changed_paths =
            crate::services::disk_reconcile::watcher_batch::collect_changed_paths(&batch);
        let disk_reconcile_state =
            app.state::<crate::services::disk_reconcile::orchestrator::DiskReconcileState>();
        let config = app.state::<crate::services::config::ConfigService>();
        let context = crate::services::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: &pool,
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: suppressor.clone(),
        };

        // Disk Reconcile only. Watcher must never invoke the Deep Match Scanner pipeline.
        let result = if events_lost {
            crate::services::disk_reconcile::orchestrator::reconcile_disk_state(
                context,
                crate::services::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                    game_id.clone(),
                    crate::services::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                    Vec::new(),
                    true,
                ),
            )
            .await
        } else {
            crate::services::disk_reconcile::orchestrator::reconcile_disk_state_from_watcher_batch(
                context,
                game_id.clone(),
                changed_paths,
                &batch,
            )
            .await
        };

        match result {
            Ok(result) => {
                let _ = app.emit("disk_reconcile:result", result);
            }
            Err(error) => {
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        error: error.to_string(),
                        path: Some(mods_path_root.clone()),
                    },
                );
            }
        }
    }

    log::info!("Watcher event loop ended for {}", mods_path_root);
}
