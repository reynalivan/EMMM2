//! Watcher lifecycle management.
//!
//! Handles starting, stopping, and configuring the filesystem watcher,
//! including the background DB-sync event loop.

use crate::services::scanner::watcher::{watch_mod_directory, ModWatchEvent, WatcherState};
use std::path::Path;
use tauri::Emitter;

/// Helper function to detect if a folder rename represents an enable/disable status change.
/// Returns Some((old_status, new_status)) if status changed, None otherwise.
fn detect_status_change(from_path: &Path, to_path: &Path) -> Option<(&'static str, &'static str)> {
    use crate::services::scanner::core::normalizer;

    let old_name = from_path.file_name()?.to_str()?;
    let new_name = to_path.file_name()?.to_str()?;

    let old_disabled = normalizer::is_disabled_folder(old_name);
    let new_disabled = normalizer::is_disabled_folder(new_name);

    if old_disabled != new_disabled {
        let old_status = if old_disabled { "DISABLED" } else { "ENABLED" };
        let new_status = if new_disabled { "DISABLED" } else { "ENABLED" };
        Some((old_status, new_status))
    } else {
        None
    }
}

/// Helper function to extract the primary path from a ModWatchEvent.
fn extract_path_from_event(event: &ModWatchEvent) -> String {
    match event {
        ModWatchEvent::Created(p) => p.clone(),
        ModWatchEvent::Modified(p) => p.clone(),
        ModWatchEvent::Removed(p) => p.clone(),
        ModWatchEvent::Renamed { from: _, to } => to.clone(),
        ModWatchEvent::StatusChanged { path, .. } => path.clone(),
        ModWatchEvent::Error(e) => e.clone(),
    }
}

/// Error structure for failed watcher sync operations.
#[derive(Debug, Clone)]
struct WatcherSyncError {
    pub event_type: String,
    pub path: String,
    pub error: String,
    pub retry_count: u32,
}

/// Sync a single watcher event with retry logic and exponential backoff.
async fn sync_watcher_event_with_retry(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    event: &ModWatchEvent,
    max_retries: u32,
) -> Result<(), WatcherSyncError> {
    let mut retry_count = 0;

    loop {
        match sync_watcher_event(pool, game_id, mods_path, event).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                retry_count += 1;

                if retry_count >= max_retries {
                    return Err(WatcherSyncError {
                        event_type: format!("{:?}", event),
                        path: extract_path_from_event(event),
                        error: e,
                        retry_count,
                    });
                }

                // Exponential backoff: 100ms, 200ms, 400ms...
                let delay = std::time::Duration::from_millis(100 * 2_u64.pow(retry_count - 1));
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Process a batch of watcher events.
/// Returns Ok(()) if all events succeeded, or Err(vec_of_failed_events) if any failed.
async fn sync_watcher_event_batch(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    events: &[ModWatchEvent],
) -> Result<(), Vec<WatcherSyncError>> {
    let mut failed_events = Vec::new();

    // Process each event with retry logic
    for event in events {
        match sync_watcher_event_with_retry(pool, game_id, mods_path, event, 3).await {
            Ok(_) => {}
            Err(e) => failed_events.push(e),
        }
    }

    if failed_events.is_empty() {
        Ok(())
    } else {
        Err(failed_events)
    }
}

/// Start the file watcher for a given path, stopping any existing watcher first.
/// Spawns a background thread that syncs DB on events and emits `mod_watch:event` to the frontend.
pub fn start_watcher(
    app: tauri::AppHandle,
    state: &WatcherState,
    pool: sqlx::SqlitePool,
    path: String,
    game_id: String,
) -> Result<(), String> {
    let path_obj = Path::new(&path);

    // Stop existing watcher
    {
        let mut w = state.watcher.lock().unwrap();
        if w.is_some() {
            log::info!("Stopping existing watcher");
            *w = None;
        }
    }

    log::info!("Starting watcher on: {}", path);

    // Auto-GC on watcher start: cleanup orphan objects deleted while app was closed
    // This handles the case where user deletes object folders during app restart/shutdown.
    let gc_pool = pool.clone();
    let gc_game_id = game_id.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            crate::services::objects::query::gc_lost_objects(&gc_pool, &gc_game_id).await
        {
            log::warn!("Auto-GC failed for game '{}': {}", gc_game_id, e);
        }
    });

    // Start new watcher
    let (watcher, rx) = watch_mod_directory(path_obj, state.suppressor.clone())?;

    // Store watcher immediately to keep it alive
    {
        let mut w = state.watcher.lock().unwrap();
        *w = Some(watcher);
    }

    // Spawn thread to handle events
    let app_handle = app.clone();
    let db_pool = pool;
    let mods_path_root = path.clone();

    std::thread::spawn(move || {
        // Imp 10: drain-and-batch pattern for rapid event bursts
        while let Ok(first) = rx.recv() {
            let mut batch = vec![first];

            // Drain any additional events that arrived during processing
            while let Ok(ev) = rx.try_recv() {
                batch.push(ev);
            }

            // 1. Sync DB mirror for all events in batch with retry logic
            let sync_result = tauri::async_runtime::block_on(async {
                sync_watcher_event_batch(&db_pool, &game_id, Path::new(&mods_path_root), &batch)
                    .await
            });

            // Emit error events for failed syncs
            if let Err(failed_events) = sync_result {
                for error in failed_events {
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({
                            "type": "Error",
                            "error": format!("Failed to sync {}: {}", error.event_type, error.error),
                            "path": error.path,
                            "retry_count": error.retry_count
                        }),
                    );
                }
            }

            // 2. Notify frontend for all events in batch
            for event in &batch {
                match event {
                    ModWatchEvent::Created(p) => {
                        let _ = app_handle.emit(
                            "mod_watch:event",
                            serde_json::json!({ "type": "Created", "path": p }),
                        );
                    }
                    ModWatchEvent::Modified(p) => {
                        let _ = app_handle.emit(
                            "mod_watch:event",
                            serde_json::json!({ "type": "Modified", "path": p }),
                        );
                    }
                    ModWatchEvent::Removed(p) => {
                        let _ = app_handle.emit(
                            "mod_watch:event",
                            serde_json::json!({ "type": "Removed", "path": p }),
                        );
                    }
                    ModWatchEvent::Renamed { from, to } => {
                        // Check if this is a status change operation
                        let from_path = Path::new(from);
                        let to_path = Path::new(to);
                        if let Some((from_status, to_status)) =
                            detect_status_change(from_path, to_path)
                        {
                            // Emit StatusChanged event for enable/disable operations
                            let _ = app_handle.emit(
                                "mod_watch:event",
                                serde_json::json!({
                                    "type": "StatusChanged",
                                    "path": to,
                                    "from_status": from_status,
                                    "to_status": to_status
                                }),
                            );
                        } else {
                            // Regular rename
                            let _ = app_handle.emit(
                                "mod_watch:event",
                                serde_json::json!({ "type": "Renamed", "from": from, "to": to }),
                            );
                        }
                    }
                    ModWatchEvent::Error(e) => {
                        let _ = app_handle.emit(
                            "mod_watch:event",
                            serde_json::json!({ "type": "Error", "error": e }),
                        );
                    }
                    _ => {}
                }
            }
        }
        log::info!("Watcher event loop ended for {}", path);
    });

    Ok(())
}

/// Core sync logic for a single watcher event.
async fn sync_watcher_event(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    event: &ModWatchEvent,
) -> Result<(), String> {
    match event {
        ModWatchEvent::Created(p) => {
            let p_obj = Path::new(&p);
            // Imp 7: use extension check instead of is_dir() syscall.
            // Directories have no extension; files always have one (filtered by translate_event).
            if p_obj.extension().is_none() {
                if let Ok(rel) = p_obj.strip_prefix(mods_path) {
                    let components: Vec<_> = rel.components().collect();
                    if components.len() == 2 {
                        // Depth 2 = Mod folder (Mods/Object/Mod)
                        let object_name = components[0].as_os_str().to_string_lossy().to_string();
                        let folder_name = components[1].as_os_str().to_string_lossy().to_string();
                        let relative_folder_path = rel.to_string_lossy().to_string();

                        let id = crate::services::scanner::sync::helpers::generate_stable_id(
                            game_id,
                            &relative_folder_path,
                        );
                        let is_enabled =
                            !crate::services::scanner::core::normalizer::is_disabled_folder(
                                &folder_name,
                            );
                        let current_status = if is_enabled { "ENABLED" } else { "DISABLED" };

                        crate::database::mod_repo::insert_new_mod(
                            pool,
                            &id,
                            game_id,
                            &folder_name,
                            &relative_folder_path,
                            current_status,
                        )
                        .await
                        .map_err(|e| format!("Failed to insert mod: {}", e))?;

                        if let Ok(mut tx) = pool.begin().await {
                            let mut new_obj = 0;
                            if let Ok(object_id) =
                                crate::services::scanner::sync::helpers::ensure_object_exists(
                                    &mut tx,
                                    game_id,
                                    &object_name,
                                    &object_name,
                                    "Other",
                                    None,
                                    "[]",
                                    "{}",
                                    &mut new_obj,
                                )
                                .await
                            {
                                let _ = tx.commit().await;
                                crate::database::mod_repo::set_mod_object(pool, &id, &object_id)
                                    .await
                                    .map_err(|e| format!("Failed to set mod object: {}", e))?;
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        ModWatchEvent::Renamed { from, to } => {
            let from_path = Path::new(&from);
            let to_path = Path::new(&to);
            if let (Ok(rel_from), Ok(rel_to)) = (
                from_path.strip_prefix(mods_path),
                to_path.strip_prefix(mods_path),
            ) {
                let comp_from: Vec<_> = rel_from.components().collect();
                let comp_to: Vec<_> = rel_to.components().collect();

                // Check for status change (enable/disable operation)
                if let Some((_from_status, to_status)) = detect_status_change(from_path, to_path) {
                    // This is an enable/disable operation - handle specially
                    if comp_from.len() == 1 && comp_to.len() == 1 {
                        // Top-level object toggle
                        let old_folder = comp_from[0].as_os_str().to_string_lossy().to_string();
                        let new_folder = comp_to[0].as_os_str().to_string_lossy().to_string();

                        // Update object folder_path
                        crate::database::object_repo::update_object_folder_path(
                            pool,
                            game_id,
                            &old_folder,
                            &new_folder,
                        )
                        .await
                        .map_err(|e| format!("Failed to update object folder path: {}", e))?;

                        // Update all child mods with new prefix
                        let old_prefix = format!("{}\\", old_folder);
                        let new_prefix = format!("{}\\", new_folder);
                        let old_prefix_fwd = format!("{}/", old_folder);
                        let new_prefix_fwd = format!("{}/", new_folder);

                        crate::database::mod_repo::update_child_paths(
                            pool,
                            game_id,
                            &old_prefix,
                            &new_prefix,
                        )
                        .await
                        .map_err(|e| format!("Failed to update child paths (backslash): {}", e))?;

                        crate::database::mod_repo::update_child_paths(
                            pool,
                            game_id,
                            &old_prefix_fwd,
                            &new_prefix_fwd,
                        )
                        .await
                        .map_err(|e| format!("Failed to update child paths (slash): {}", e))?;

                        // Update status for all mods in this object
                        crate::database::mod_repo::update_status_for_object(
                            pool,
                            game_id,
                            &new_folder,
                            to_status,
                        )
                        .await
                        .map_err(|e| format!("Failed to update object status: {}", e))?;

                        return Ok(());
                    }
                }

                // Regular rename (no status change or mod-level rename)
                if comp_from.len() == 2 && comp_to.len() == 2 {
                    let old_rel = rel_from.to_string_lossy().to_string();
                    let new_rel = rel_to.to_string_lossy().to_string();
                    let new_folder_name = comp_to[1].as_os_str().to_string_lossy().to_string();
                    let is_enabled =
                        !crate::services::scanner::core::normalizer::is_disabled_folder(
                            &new_folder_name,
                        );
                    let new_status = if is_enabled { "ENABLED" } else { "DISABLED" };
                    let new_id = crate::services::scanner::sync::helpers::generate_stable_id(
                        game_id, &new_rel,
                    );

                    crate::database::mod_repo::update_mod_identity(
                        pool,
                        &new_id,
                        &new_rel,
                        &new_folder_name,
                        new_status,
                        &old_rel,
                        game_id,
                    )
                    .await
                    .map_err(|e| format!("Failed to update mod identity: {}", e))?;

                    // Ensure that new object folder is registered
                    let object_name = comp_to[0].as_os_str().to_string_lossy().to_string();
                    if let Ok(mut tx) = pool.begin().await {
                        let mut new_obj = 0;
                        if let Ok(object_id) =
                            crate::services::scanner::sync::helpers::ensure_object_exists(
                                &mut tx,
                                game_id,
                                &object_name,
                                &object_name,
                                "Other",
                                None,
                                "[]",
                                "{}",
                                &mut new_obj,
                            )
                            .await
                        {
                            let _ = tx.commit().await;
                            crate::database::mod_repo::set_mod_object(pool, &new_id, &object_id)
                                .await
                                .map_err(|e| format!("Failed to set mod object: {}", e))?;
                        }
                    }
                }
            }
            Ok(())
        }
        ModWatchEvent::Removed(p) => {
            let p_obj = Path::new(&p);
            if let Ok(rel) = p_obj.strip_prefix(mods_path) {
                let components: Vec<_> = rel.components().collect();
                if components.len() == 2 {
                    // Depth 2: a mod folder was deleted — delete only the mod row
                    let rel_path = rel.to_string_lossy().to_string();
                    crate::database::mod_repo::delete_mod_by_path_and_game(
                        pool, &rel_path, game_id,
                    )
                    .await
                    .map_err(|e| format!("Failed to delete mod: {}", e))?;
                } else if components.len() == 1 {
                    // Depth 1: an entire object folder was deleted — atomically
                    // remove the object and all its child mods from the DB.
                    let folder_name = components[0].as_os_str().to_string_lossy().to_string();
                    log::info!(
                        "Watcher: object folder '{}' removed for game '{}' — purging from DB",
                        folder_name,
                        game_id
                    );
                    crate::database::object_repo::delete_object_and_mods_by_folder(
                        pool,
                        game_id,
                        &folder_name,
                    )
                    .await
                    .map_err(|e| {
                        format!("Failed to delete object folder '{}': {}", folder_name, e)
                    })?;
                }
            }
            Ok(())
        }
        ModWatchEvent::Modified(_) => {
            // Modified events are handled at frontend level (mark as stale, invalidate thumbnails)
            // No DB update needed for modified files
            Ok(())
        }
        ModWatchEvent::StatusChanged { path, .. } => {
            // StatusChanged events are handled in Renamed handler
            // This is a fallback if event is emitted directly
            log::warn!(
                "StatusChanged event received unexpectedly for path: {}",
                path
            );
            Ok(())
        }
        ModWatchEvent::Error(e) => Err(format!("Watcher error: {}", e)),
    }
}
