//! Watcher lifecycle management.
//!
//! Handles starting, stopping, and configuring the filesystem watcher,
//! including the async DB-sync event loop.
//!
//! # Architecture (post-refactor)
//!
//! - Pure async event loop via `tokio::spawn` (no `std::thread` + `block_on`)
//! - Typed IPC payloads via `WatchEventPayload` (no `serde_json::json!`)
//! - DRY helpers for repeated rename/path-update logic

use crate::services::scanner::watcher::{ModWatchEvent, WatchEventPayload, WatcherState};
use std::path::Path;
use tauri::Emitter;

// ── Helper: rename an object folder and update all child mod paths ─────

/// Renames an object folder in the DB and cascades path changes to all child mods.
/// Optionally updates the enable/disable status of all child mods.
async fn rename_object_folder(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    old_folder: &str,
    new_folder: &str,
    new_status: Option<&str>,
) -> Result<(), String> {
    // 1. Update object's folder_path
    crate::database::object_repo::update_object_folder_path(
        &mut *conn, game_id, old_folder, new_folder,
    )
    .await
    .map_err(|e| format!("Failed to update object folder path: {}", e))?;

    // 2. Cascade path changes to all child mods (both separator styles)
    for (old_sep, new_sep) in [
        (format!("{}\\", old_folder), format!("{}\\", new_folder)),
        (format!("{}/", old_folder), format!("{}/", new_folder)),
    ] {
        crate::database::mod_repo::update_child_paths(&mut *conn, game_id, &old_sep, &new_sep)
            .await
            .map_err(|e| format!("Failed to update child paths: {}", e))?;
    }

    // 3. Optionally update status for all child mods
    if let Some(status) = new_status {
        crate::database::mod_repo::update_status_for_object(
            &mut *conn, game_id, new_folder, status,
        )
        .await
        .map_err(|e| format!("Failed to update object status: {}", e))?;
    }

    Ok(())
}

// ── Helper: emit a typed event to the frontend ─────────────────────────

/// Emit a `WatchEventPayload` to the frontend via `mod_watch:event`.
fn emit_event(app: &tauri::AppHandle, payload: WatchEventPayload) {
    let _ = app.emit("mod_watch:event", payload);
}

// ── Error tracking ─────────────────────────────────────────────────────

/// Error structure for failed watcher sync operations.
#[derive(Debug, Clone)]
struct WatcherSyncError {
    pub event_type: String,
    pub path: String,
    pub error: String,
    pub retry_count: u32,
}

// ── Retry logic ────────────────────────────────────────────────────────

/// Sync a single watcher event with retry logic and exponential backoff.
async fn sync_watcher_event_with_retry(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &Path,
    event: &ModWatchEvent,
    max_retries: u32,
) -> Result<(), WatcherSyncError> {
    let mut retry_count = 0;

    loop {
        match sync_watcher_event(&mut *conn, game_id, mods_path, event).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                retry_count += 1;

                if retry_count >= max_retries {
                    return Err(WatcherSyncError {
                        event_type: format!("{:?}", event),
                        path: extract_path(event),
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

/// Extract the primary path from a ModWatchEvent for error reporting.
fn extract_path(event: &ModWatchEvent) -> String {
    match event {
        ModWatchEvent::Created(p)
        | ModWatchEvent::Modified(p)
        | ModWatchEvent::Removed(p)
        | ModWatchEvent::StatusChanged { path: p, .. } => p.clone(),
        ModWatchEvent::Renamed { to, .. } => to.clone(),
        ModWatchEvent::Error(e) => e.clone(),
    }
}

/// Process a batch of watcher events with retry logic.
/// Returns `Ok(())` if all succeeded, or `Err(failed)` for partial failures.
async fn sync_watcher_event_batch(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    events: &[ModWatchEvent],
) -> Result<(), Vec<WatcherSyncError>> {
    let mut failed = Vec::new();

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            failed.push(WatcherSyncError {
                event_type: "BatchTxStart".to_string(),
                path: "".to_string(),
                error: format!("Failed to start DB transaction: {}", e),
                retry_count: 0,
            });
            return Err(failed);
        }
    };

    for event in events {
        if let Err(e) = sync_watcher_event_with_retry(&mut *tx, game_id, mods_path, event, 3).await
        {
            failed.push(e);
        }
    }

    if failed.is_empty() {
        if let Err(e) = tx.commit().await {
            failed.push(WatcherSyncError {
                event_type: "BatchTxCommit".to_string(),
                path: "".to_string(),
                error: format!("Failed to commit DB transaction: {}", e),
                retry_count: 0,
            });
            return Err(failed);
        }
        Ok(())
    } else {
        let _ = tx.rollback().await;
        Err(failed)
    }
}

// ── Watcher Lifecycle ──────────────────────────────────────────────────

/// Start the file watcher for a given path, stopping any existing watcher first.
/// Spawns a `tokio::spawn` task that syncs DB on events and emits typed payloads to the frontend.
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

    // Auto-GC: cleanup orphan objects deleted while app was closed.
    // Runs on a background tokio task (not fire-and-forget — errors are logged).
    let gc_pool = pool.clone();
    let gc_game_id = game_id.clone();
    tokio::spawn(async move {
        if let Err(e) =
            crate::services::objects::query::gc_lost_objects(&gc_pool, &gc_game_id).await
        {
            log::warn!("Auto-GC failed for game '{}': {}", gc_game_id, e);
        }
    });

    // Start new watcher (returns tokio::sync::mpsc::UnboundedReceiver)
    let (watcher, rx) =
        crate::services::scanner::watcher::watch_mod_directory(path_obj, state.suppressor.clone())?;

    // Store watcher handle to keep it alive
    {
        let mut w = state.watcher.lock().unwrap();
        *w = Some(watcher);
    }

    // Spawn async event loop
    let app_handle = app.clone();
    let db_pool = pool;
    let mods_path_root = path.clone();

    tokio::spawn(async move {
        process_event_loop(rx, app_handle, db_pool, game_id, mods_path_root).await;
    });

    Ok(())
}

/// Async event loop: drains batches from the receiver, syncs DB,
/// then emits typed payloads to the frontend.
async fn process_event_loop(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<ModWatchEvent>,
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
    mods_path_root: String,
) {
    loop {
        let mut batch = Vec::new();

        // 1. Wait for the first event in the batch
        let first_ev = match rx.recv().await {
            Some(ev) => ev,
            None => break, // Channel closed
        };
        batch.push(first_ev);

        // 2. Dynamic Debounce Window (Max 1000ms, flushes if 50ms silence)
        let max_wait = tokio::time::sleep(std::time::Duration::from_millis(1000));
        tokio::pin!(max_wait);

        loop {
            let silence_timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
            tokio::select! {
                _ = &mut max_wait => {
                    break; // Hit 1s max wait limit, flush now
                }
                _ = silence_timeout => {
                    break; // 50ms of silence, flush now
                }
                ev_opt = rx.recv() => {
                    if let Some(ev) = ev_opt {
                        batch.push(ev);
                    } else {
                        break; // Channel closed
                    }
                }
            }
        }

        log::debug!(
            "Watcher logic: flushing batched events, count = {}",
            batch.len()
        );

        // 3. Process DB Sync in a single Transaction Wrapper
        let sync_result =
            sync_watcher_event_batch(&pool, &game_id, Path::new(&mods_path_root), &batch).await;

        // Emit error payloads for failed syncs
        if let Err(failed) = sync_result {
            for error in failed {
                emit_event(
                    &app,
                    WatchEventPayload::Error {
                        error: format!("Failed to sync {}: {}", error.event_type, error.error),
                        path: Some(error.path),
                        retry_count: Some(error.retry_count),
                    },
                );
            }
        }

        // 4. Emit typed payloads to frontend
        let mut emit_batch = Vec::new();
        for event in &batch {
            emit_batch.push(WatchEventPayload::from_event(event));
        }

        if !emit_batch.is_empty() {
            let _ = app.emit("mod_watch:events_batch", emit_batch);
        }
    }

    log::info!("Watcher event loop ended for {}", mods_path_root);
}

// ── Core DB Sync ───────────────────────────────────────────────────────

/// Sync a single watcher event to the database.
async fn sync_watcher_event(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_path: &Path,
    event: &ModWatchEvent,
) -> Result<(), String> {
    match event {
        ModWatchEvent::Created(p) => {
            let p_obj = Path::new(&p);
            // Directories have no extension; files always have one (filtered upstream).
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
                            &mut *conn,
                            &id,
                            game_id,
                            &folder_name,
                            &relative_folder_path,
                            current_status,
                        )
                        .await
                        .map_err(|e| format!("Failed to insert mod: {}", e))?;

                        let mut new_obj = 0;
                        if let Ok(object_id) =
                            crate::services::scanner::sync::helpers::ensure_object_exists(
                                &mut *conn,
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
                            crate::database::mod_repo::set_mod_object(&mut *conn, &id, &object_id)
                                .await
                                .map_err(|e| format!("Failed to set mod object: {}", e))?;
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

                if comp_from.len() == 1 && comp_to.len() == 1 {
                    // Depth 1: Object folder rename (e.g. Alhaitham → Alhaitham2)
                    let old_folder = comp_from[0].as_os_str().to_string_lossy().to_string();
                    let new_folder = comp_to[0].as_os_str().to_string_lossy().to_string();
                    rename_object_folder(&mut *conn, game_id, &old_folder, &new_folder, None)
                        .await?;
                } else if comp_from.len() == 2 && comp_to.len() == 2 {
                    // Depth 2: Mod folder rename
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
                        &mut *conn,
                        &new_id,
                        &new_rel,
                        &new_folder_name,
                        new_status,
                        &old_rel,
                        game_id,
                    )
                    .await
                    .map_err(|e| format!("Failed to update mod identity: {}", e))?;

                    // Ensure new object folder is registered
                    let object_name = comp_to[0].as_os_str().to_string_lossy().to_string();
                    let mut new_obj = 0;
                    if let Ok(object_id) =
                        crate::services::scanner::sync::helpers::ensure_object_exists(
                            &mut *conn,
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
                        crate::database::mod_repo::set_mod_object(&mut *conn, &new_id, &object_id)
                            .await
                            .map_err(|e| format!("Failed to set mod object: {}", e))?;
                    }
                }
            }
            Ok(())
        }

        ModWatchEvent::StatusChanged {
            path: _,
            from_status: _,
            to_status,
        } => {
            // Status change is a rename that changed the DISABLED prefix.
            // The rename itself is handled via the event's original `from`/`to` paths,
            // but we also need to do the folder-level rename + status update.
            // Note: StatusChanged events are created by mod.rs from Renamed events,
            // so the path here is the `to` path (new name).
            // We need to reconstruct the `from` path — but we don't have it here.
            // For status-only changes, the object folder name (minus prefix) stays the same,
            // so we handle this by looking up the existing DB record.
            //
            // In practice, StatusChanged at depth 1 is handled in `Renamed` arm above
            // when detect_status_change returns Some. This arm is a safety net.
            log::debug!(
                "StatusChanged event (to_status={}) handled via rename pipeline",
                to_status
            );
            Ok(())
        }

        ModWatchEvent::Removed(p) => {
            let p_obj = Path::new(&p);
            if let Ok(rel) = p_obj.strip_prefix(mods_path) {
                let components: Vec<_> = rel.components().collect();
                if components.len() == 2 {
                    // Depth 2: mod folder deleted
                    let rel_path = rel.to_string_lossy().to_string();
                    crate::database::mod_repo::delete_mod_by_path_and_game(
                        &mut *conn, &rel_path, game_id,
                    )
                    .await
                    .map_err(|e| format!("Failed to delete mod: {}", e))?;
                } else if components.len() == 1 {
                    // Depth 1: entire object folder deleted
                    let folder_name = components[0].as_os_str().to_string_lossy().to_string();
                    log::info!(
                        "Watcher: object folder '{}' removed for game '{}' — purging from DB",
                        folder_name,
                        game_id
                    );
                    crate::database::object_repo::delete_object_and_mods_by_folder(
                        &mut *conn,
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
            // Modified events are handled at frontend level (mark stale, invalidate thumbnails).
            // No DB update needed for content changes.
            Ok(())
        }

        ModWatchEvent::Error(e) => Err(format!("Watcher error: {}", e)),
    }
}
