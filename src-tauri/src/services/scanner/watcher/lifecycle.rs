//! Watcher lifecycle management.
//!
//! Handles starting, stopping, and configuring the filesystem watcher,
//! including the background DB-sync event loop.

use crate::services::scanner::watcher::{watch_mod_directory, ModWatchEvent, WatcherState};
use std::path::Path;
use tauri::Emitter;

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
        while let Ok(event) = rx.recv() {
            // 1. Sync DB mirror
            tauri::async_runtime::block_on(sync_watcher_event(
                &db_pool,
                &game_id,
                Path::new(&mods_path_root),
                &event,
            ));

            // 2. Notify frontend to refresh queries
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
                    let _ = app_handle.emit(
                        "mod_watch:event",
                        serde_json::json!({ "type": "Renamed", "from": from, "to": to }),
                    );
                }
                ModWatchEvent::Error(e) => {
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

async fn sync_watcher_event(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    event: &ModWatchEvent,
) {
    match event {
        ModWatchEvent::Created(p) => {
            let p_obj = Path::new(&p);
            if p_obj.is_dir() {
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

                        let _ = crate::database::mod_repo::insert_new_mod(
                            pool,
                            &id,
                            game_id,
                            &folder_name,
                            &relative_folder_path,
                            current_status,
                        )
                        .await;

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
                                let _ = crate::database::mod_repo::set_mod_object(
                                    pool, &id, &object_id,
                                )
                                .await;
                            }
                        }
                    }
                }
            }
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

                    let _ = crate::database::mod_repo::update_mod_identity(
                        pool,
                        &new_id,
                        &new_rel,
                        &new_folder_name,
                        new_status,
                        &old_rel,
                        game_id,
                    )
                    .await;

                    // Ensure the new object folder is registered
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
                            let _ = crate::database::mod_repo::set_mod_object(
                                pool, &new_id, &object_id,
                            )
                            .await;
                        }
                    }
                }
            }
        }
        ModWatchEvent::Removed(p) => {
            let p_obj = Path::new(&p);
            if let Ok(rel) = p_obj.strip_prefix(mods_path) {
                let components: Vec<_> = rel.components().collect();
                if components.len() == 2 {
                    let rel_path = rel.to_string_lossy().to_string();
                    let _ = crate::database::mod_repo::delete_mod_by_path_and_game(
                        pool, &rel_path, game_id,
                    )
                    .await;
                }
            }
        }
        _ => {}
    }
}
