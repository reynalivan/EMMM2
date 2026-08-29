//! Watcher lifecycle management.
//!
//! The watcher is now a pure trigger source:
//! - collect filesystem events
//! - debounce into batches
//! - delegate Disk Reconcile to `disk_reconcile`
//! - emit typed payloads back to the frontend

use crate::shared::sync::lock;
use crate::shared::errors::ScannerError;
use crate::modules::workspace::application::scanner::watcher::{
    ModWatchEvent, WatchEventPayload, WatcherSession, WatcherState, WatcherSuppressor,
};
use std::sync::Arc;
use tauri::{Emitter, Manager};

fn emit_event(app: &tauri::AppHandle, payload: WatchEventPayload) {
    let _ = app.emit("mod_watch:event", payload);
}

fn replace_watcher(
    state: &WatcherState,
    root: &std::path::Path,
    build: impl FnOnce(
        WatcherSession,
    ) -> Result<
        (
            crate::modules::workspace::application::scanner::watcher::ModWatcher,
            crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        ),
        ScannerError,
    >,
) -> Result<
    (
        WatcherSession,
        crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
    ),
    ScannerError,
> {
    let mut active_watcher = lock(&state.watcher);
    let session = state.prepare_session(root);
    let (watcher, receiver) = build(session.clone())?;
    state.publish_session(&session);
    if active_watcher.is_some() {
        log::info!("Stopping existing watcher");
    }
    *active_watcher = Some(watcher);
    Ok((session, receiver))
}

pub fn start_watcher(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
) -> Result<(), ScannerError> {
    let path_obj = std::path::Path::new(&path);

    log::info!("Starting watcher on: {}", path);

    let (session, rx) = replace_watcher(state, path_obj, |session| {
        crate::modules::workspace::application::scanner::watcher::watch_mod_directory(
            path_obj,
            state.suppressor.clone(),
            session,
        )
    })?;

    let app_handle = app.clone();
    let db_pool = pool;
    let mods_path_root = path;
    let suppressor = state.suppressor.clone();

    tokio::spawn(async move {
        process_event_loop(
            rx,
            app_handle,
            db_pool,
            game_id,
            mods_path_root,
            suppressor,
            session,
        )
        .await;
    });

    Ok(())
}

async fn process_event_loop(
    mut rx: crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
    mods_path_root: String,
    suppressor: Arc<WatcherSuppressor>,
    session: WatcherSession,
) {
    if !app.state::<WatcherState>().is_current_session(&session) {
        return;
    }
    // A watcher restart creates an event-history gap by definition (app boot,
    // drive reconnect, webview reload, or explicit stop/start). Verify the
    // whole source before trusting the first scoped event from this session.
    let disk_reconcile_state =
        app.state::<crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState>();
    let config = app.state::<crate::modules::system::application::config::ConfigService>();
    let operation_lock = app.state::<crate::platform::fs::operation_lock::OperationLock>();
    let session_recovery = crate::modules::workspace::application::disk_reconcile::orchestrator::reconcile_disk_state(
        crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: &pool,
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: suppressor.clone(),
            operation_lock: operation_lock.inner(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id.clone(),
                    crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                ),
            )),
        },
        crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
            game_id.clone(),
            crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
            Vec::new(),
            true,
        )
        .for_watcher_session(session.clone()),
    )
    .await;
    match session_recovery {
        Ok(result) if app.state::<WatcherState>().is_current_session(&session) => {
            let _ = app.emit("disk_reconcile:result", result);
        }
        Err(error) if app.state::<WatcherState>().is_current_session(&session) => emit_event(
            &app,
            WatchEventPayload::Error {
                game_id: game_id.clone(),
                error: error.to_string(),
                path: Some(mods_path_root.clone()),
            },
        ),
        _ => return,
    }

    loop {
        // The debouncer already batches (one callback per debounce window and
        // it sends its whole batch synchronously), so a recv + drain
        // reassembles it without extra timers here.
        let mut batch = Vec::new();
        let Some(first_event) = rx.recv().await else {
            break;
        };
        if !app.state::<WatcherState>().is_current_session(&session) {
            break;
        }
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
            crate::modules::workspace::application::disk_reconcile::watcher_batch::collect_changed_paths(&batch);
        let disk_reconcile_state =
            app.state::<crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let config = app.state::<crate::modules::system::application::config::ConfigService>();
        let operation_lock =
            app.state::<crate::platform::fs::operation_lock::OperationLock>();
        let context = crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileContext {
            pool: &pool,
            config: config.inner(),
            state: disk_reconcile_state.inner(),
            watcher_suppressor: suppressor.clone(),
            operation_lock: operation_lock.inner(),
            progress_reporter: Some(std::sync::Arc::new(
                crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter::new(
                    app.clone(),
                    game_id.clone(),
                    if events_lost {
                        crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::ManualRepair
                    } else {
                        crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::WatcherBatch
                    },
                ),
            )),
        };

        // Disk Reconcile only. Watcher must never invoke the Deep Match Scanner pipeline.
        let result = if events_lost {
            crate::modules::workspace::application::disk_reconcile::orchestrator::reconcile_disk_state(
                context,
                crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                    game_id.clone(),
                    crate::modules::workspace::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                    Vec::new(),
                    true,
                )
                .for_watcher_session(session.clone()),
            )
            .await
        } else {
            crate::modules::workspace::application::disk_reconcile::orchestrator::reconcile_disk_state_from_watcher_batch(
                context,
                game_id.clone(),
                changed_paths,
                &batch,
                session.clone(),
            )
            .await
        };

        if !app.state::<WatcherState>().is_current_session(&session) {
            break;
        }
        match result {
            Ok(result) => {
                let _ = app.emit("disk_reconcile:result", result);
            }
            Err(error) => {
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        game_id: game_id.clone(),
                        error: error.to_string(),
                        path: Some(mods_path_root.clone()),
                    },
                );
            }
        }
    }

    log::info!("Watcher event loop ended for {}", mods_path_root);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    async fn assert_created_event(
        receiver: &mut crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        expected: &Path,
    ) {
        std::fs::write(expected, "content").expect("write watched file");
        let expected = expected.to_string_lossy().to_string();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match receiver.recv().await {
                    Some(ModWatchEvent::Created(path)) if path == expected => break,
                    Some(_) => continue,
                    None => panic!("installed watcher event channel closed"),
                }
            }
        })
        .await
        .expect("installed watcher should deliver created event");
    }

    fn build_real_watcher(
        state: &WatcherState,
        root: &Path,
        session: WatcherSession,
    ) -> Result<
        (
            crate::modules::workspace::application::scanner::watcher::ModWatcher,
            crate::modules::workspace::application::scanner::watcher::WatchEventReceiver,
        ),
        ScannerError,
    > {
        crate::modules::workspace::application::scanner::watcher::watch_mod_directory(
            root,
            state.suppressor.clone(),
            session,
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_starts_keep_the_latest_installed_session_delivering_events() {
        let first_dir = tempfile::tempdir().expect("first tempdir");
        let second_dir = tempfile::tempdir().expect("second tempdir");
        let first_root = first_dir.path().to_path_buf();
        let second_root = second_dir.path().to_path_buf();
        let state = Arc::new(WatcherState::new());
        let first_build_started = Arc::new(Barrier::new(2));
        let release_first_build = Arc::new(Barrier::new(2));
        let second_start_attempted = Arc::new(Barrier::new(2));

        let first_handle = {
            let state = state.clone();
            let root = first_root.clone();
            let started = first_build_started.clone();
            let release = release_first_build.clone();
            std::thread::spawn(move || {
                replace_watcher(&state, &root, |session| {
                    started.wait();
                    release.wait();
                    build_real_watcher(&state, &root, session)
                })
            })
        };
        first_build_started.wait();

        let second_handle = {
            let state = state.clone();
            let root = second_root.clone();
            let attempted = second_start_attempted.clone();
            std::thread::spawn(move || {
                attempted.wait();
                replace_watcher(&state, &root, |session| {
                    build_real_watcher(&state, &root, session)
                })
            })
        };
        second_start_attempted.wait();
        release_first_build.wait();

        let (first_session, _first_receiver) = first_handle
            .join()
            .expect("first start thread")
            .expect("first watcher start");
        let (second_session, mut second_receiver) = second_handle
            .join()
            .expect("second start thread")
            .expect("second watcher start");

        assert!(!state.is_current_session(&first_session));
        assert!(state.is_current_session(&second_session));
        assert_created_event(&mut second_receiver, &second_root.join("latest.ini")).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_replacement_keeps_the_installed_session_delivering_events() {
        let original_dir = tempfile::tempdir().expect("original tempdir");
        let original_root = original_dir.path().to_path_buf();
        let state = Arc::new(WatcherState::new());
        let (original_session, mut original_receiver) =
            replace_watcher(&state, &original_root, |session| {
                build_real_watcher(&state, &original_root, session)
            })
            .expect("original watcher start");
        let failed_build_started = Arc::new(Barrier::new(2));
        let release_failed_build = Arc::new(Barrier::new(2));

        let failed_handle = {
            let state = state.clone();
            let started = failed_build_started.clone();
            let release = release_failed_build.clone();
            std::thread::spawn(move || {
                replace_watcher(&state, &PathBuf::from("missing-root"), |_session| {
                    started.wait();
                    release.wait();
                    Err(ScannerError::Validation(
                        "injected watcher construction failure".to_string(),
                    ))
                })
            })
        };
        failed_build_started.wait();
        release_failed_build.wait();
        let error = match failed_handle.join().expect("failed start thread") {
            Ok(_) => panic!("replacement should fail"),
            Err(error) => error,
        };

        assert!(matches!(error, ScannerError::Validation(_)));
        assert!(state.is_current_session(&original_session));
        assert_created_event(
            &mut original_receiver,
            &original_root.join("still-current.ini"),
        )
        .await;
    }
}
