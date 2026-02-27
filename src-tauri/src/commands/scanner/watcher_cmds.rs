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
    game_id: String,
    state: State<'_, WatcherState>,
    pool: State<'_, sqlx::SqlitePool>,
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
    let db_pool = (*pool).clone();
    let mods_path_root = path.clone();

    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            // 1. Sync DB mirror
            tauri::async_runtime::block_on(sync_watcher_event(&db_pool, &game_id, Path::new(&mods_path_root), &event));

            // 2. Notify frontend to refresh queries
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

async fn sync_watcher_event(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    event: &crate::services::scanner::watcher::ModWatchEvent,
) {
    match event {
        crate::services::scanner::watcher::ModWatchEvent::Created(p) => {
            let p_obj = Path::new(&p);
            if p_obj.is_dir() {
                if let Ok(rel) = p_obj.strip_prefix(mods_path) {
                    let components: Vec<_> = rel.components().collect();
                    if components.len() == 2 {
                        // Depth 2 = Mod folder (Mods/Object/Mod)
                        let object_name = components[0].as_os_str().to_string_lossy().to_string();
                        let folder_name = components[1].as_os_str().to_string_lossy().to_string();
                        let relative_folder_path = rel.to_string_lossy().to_string();

                        let id = crate::services::scanner::sync::helpers::generate_stable_id(game_id, &relative_folder_path);
                        let is_enabled = !folder_name.starts_with(crate::DISABLED_PREFIX);
                        let current_status = if is_enabled { "ENABLED" } else { "DISABLED" };

                        let _ = sqlx::query(
                            "INSERT OR IGNORE INTO mods (id, game_id, actual_name, folder_path, status, object_type, is_favorite, is_safe) VALUES (?, ?, ?, ?, ?, 'Other', 0, 1)"
                        )
                        .bind(&id)
                        .bind(game_id)
                        .bind(&folder_name)
                        .bind(&relative_folder_path)
                        .bind(current_status)
                        .execute(pool)
                        .await;

                        if let Ok(mut tx) = pool.begin().await {
                            let mut new_obj = 0;
                            if let Ok(object_id) = crate::services::scanner::sync::helpers::ensure_object_exists(
                                &mut tx,
                                game_id,
                                &object_name,
                                &object_name,
                                "Other",
                                None,
                                "[]",
                                "{}",
                                &mut new_obj
                            ).await {
                                let _ = tx.commit().await;
                                let _ = sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
                                    .bind(object_id)
                                    .bind(&id)
                                    .execute(pool)
                                    .await;
                            }
                        }
                    }
                }
            }
        }
        crate::services::scanner::watcher::ModWatchEvent::Renamed { from, to } => {
            let from_path = Path::new(&from);
            let to_path = Path::new(&to);
            if let (Ok(rel_from), Ok(rel_to)) = (from_path.strip_prefix(mods_path), to_path.strip_prefix(mods_path)) {
                let comp_from: Vec<_> = rel_from.components().collect();
                let comp_to: Vec<_> = rel_to.components().collect();

                if comp_from.len() == 2 && comp_to.len() == 2 {
                    let old_rel = rel_from.to_string_lossy().to_string();
                    let new_rel = rel_to.to_string_lossy().to_string();
                    let new_folder_name = comp_to[1].as_os_str().to_string_lossy().to_string();
                    let is_enabled = !new_folder_name.starts_with(crate::DISABLED_PREFIX);
                    let new_status = if is_enabled { "ENABLED" } else { "DISABLED" };
                    let new_id = crate::services::scanner::sync::helpers::generate_stable_id(game_id, &new_rel);

                    let _ = sqlx::query("UPDATE mods SET id = ?, folder_path = ?, actual_name = ?, status = ? WHERE folder_path = ? AND game_id = ?")
                        .bind(&new_id)
                        .bind(&new_rel)
                        .bind(&new_folder_name)
                        .bind(new_status)
                        .bind(&old_rel)
                        .bind(game_id)
                        .execute(pool)
                        .await;

                    // Ensure the new object folder is registered
                    let object_name = comp_to[0].as_os_str().to_string_lossy().to_string();
                    if let Ok(mut tx) = pool.begin().await {
                        let mut new_obj = 0;
                        if let Ok(object_id) = crate::services::scanner::sync::helpers::ensure_object_exists(
                            &mut tx,
                            game_id,
                            &object_name,
                            &object_name,
                            "Other",
                            None,
                            "[]",
                            "{}",
                            &mut new_obj
                        ).await {
                            let _ = tx.commit().await;
                            let _ = sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
                                .bind(object_id)
                                .bind(&new_id)
                                .execute(pool)
                                .await;
                        }
                    }
                }
            }
        }
        crate::services::scanner::watcher::ModWatchEvent::Removed(p) => {
            let p_obj = Path::new(&p);
            if let Ok(rel) = p_obj.strip_prefix(mods_path) {
                let components: Vec<_> = rel.components().collect();
                if components.len() == 2 {
                    let rel_path = rel.to_string_lossy().to_string();
                    let _ = sqlx::query("DELETE FROM mods WHERE folder_path = ? AND game_id = ?")
                        .bind(&rel_path)
                        .bind(game_id)
                        .execute(pool)
                        .await;
                }
            }
        }
        _ => {}
    }
}
