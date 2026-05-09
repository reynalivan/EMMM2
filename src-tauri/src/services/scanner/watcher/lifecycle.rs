//! Watcher lifecycle management.
//!
//! The watcher is now a pure trigger source:
//! - collect filesystem events
//! - debounce into batches
//! - delegate Disk Reconcile to `disk_reconcile`
//! - emit typed payloads back to the frontend

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
) -> Result<(), String> {
    let path_obj = std::path::Path::new(&path);

    {
        let mut watcher = state.watcher.lock().unwrap();
        if watcher.is_some() {
            log::info!("Stopping existing watcher");
            *watcher = None;
        }
    }

    log::info!("Starting watcher on: {}", path);

    let (watcher, rx) =
        crate::services::scanner::watcher::watch_mod_directory(path_obj, state.suppressor.clone())?;

    {
        let mut active_watcher = state.watcher.lock().unwrap();
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
        let mut batch = Vec::new();

        let Some(first_event) = rx.recv().await else {
            break;
        };
        batch.push(first_event);

        let max_wait = tokio::time::sleep(std::time::Duration::from_millis(1000));
        tokio::pin!(max_wait);

        loop {
            let silence_timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
            tokio::select! {
                _ = &mut max_wait => {
                    break;
                }
                _ = silence_timeout => {
                    break;
                }
                event = rx.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    batch.push(event);
                }
            }
        }

        log::debug!("Watcher flushing batched events: {}", batch.len());

        let changed_paths =
            crate::services::disk_reconcile::watcher_batch::collect_changed_paths(&batch);
        let disk_reconcile_state =
            app.state::<crate::services::disk_reconcile::orchestrator::DiskReconcileState>();
        let config = app.state::<crate::services::config::ConfigService>();

        // Disk Reconcile only. Watcher must never invoke the Deep Match Scanner pipeline.
        match crate::services::disk_reconcile::orchestrator::reconcile_disk_state_from_watcher_batch(
            crate::services::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &pool,
                config: config.inner(),
                state: disk_reconcile_state.inner(),
                watcher_suppressor: suppressor.clone(),
            },
            game_id.clone(),
            changed_paths,
            &batch,
        )
        .await
        {
            Ok(result) => {
                let _ = app.emit("disk_reconcile:result", result);
            }
            Err(error) => {
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        error,
                        path: Some(mods_path_root.clone()),
                        retry_count: None,
                    },
                );
            }
        }

        let payloads = batch
            .iter()
            .map(WatchEventPayload::from_event)
            .collect::<Vec<_>>();

        if !payloads.is_empty() {
            let _ = app.emit("mod_watch:events_batch", payloads);
        }
    }

    log::info!("Watcher event loop ended for {}", mods_path_root);
}
