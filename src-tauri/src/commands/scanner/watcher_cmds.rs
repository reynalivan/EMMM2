//! File system watcher commands.

use crate::services::scanner::watcher::WatcherState;
use std::path::Path;
use std::sync::atomic::Ordering;
use tauri::{Emitter, State};

/// Manually set watcher suppression state (e.g. for bulk operations).
///
/// # Covers: EC-2.06
#[tauri::command]
pub async fn set_watcher_suppression_cmd(
    suppressed: bool,
    watcher: State<'_, WatcherState>,
) -> Result<(), String> {
    watcher.suppressor.store(suppressed, Ordering::Relaxed);
    Ok(())
}

/// Start the file watcher for a specific path.
/// Emits `mod_watch:event` to the frontend.
#[tauri::command]
pub async fn start_watcher_cmd(
    app: tauri::AppHandle,
    path: String,
    state: State<'_, WatcherState>,
) -> Result<(), String> {
    let path_obj = Path::new(&path);

    // Stop existing watcher
    {
        let mut w = state.watcher.lock().unwrap();
        if w.is_some() {
            log::info!("Stopping existing watcher");
            *w = None; // Drop the old watcher
        }
    }

    log::info!("Starting watcher on: {}", path);

    // Start new watcher
    let (watcher, rx) =
        crate::services::scanner::watcher::watch_mod_directory(path_obj, state.suppressor.clone())?;

    // Store watcher immediately to keep it alive
    {
        let mut w = state.watcher.lock().unwrap();
        *w = Some(watcher);
    }

    // Spawn thread to handle events
    let app_handle = app.clone();
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            // Temporary fix: Serialize manually or just basic info
            match event {
                crate::services::scanner::watcher::ModWatchEvent::Created(p) => {
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({ "type": "Created", "path": p }),
                    );
                }
                crate::services::scanner::watcher::ModWatchEvent::Modified(p) => {
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({ "type": "Modified", "path": p }),
                    );
                }
                crate::services::scanner::watcher::ModWatchEvent::Removed(p) => {
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({ "type": "Removed", "path": p }),
                    );
                }
                crate::services::scanner::watcher::ModWatchEvent::Renamed { from, to } => {
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({ "type": "Renamed", "from": from, "to": to }),
                    );
                }
                crate::services::scanner::watcher::ModWatchEvent::Error(e) => {
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({ "type": "Error", "error": e }),
                    );
                }
            }
        }
        log::info!("Watcher event loop ended for {}", path);
    });

    Ok(())
}
