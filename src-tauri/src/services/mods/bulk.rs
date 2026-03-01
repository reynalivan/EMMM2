use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
use crate::database::mod_repo;
use crate::services::mods::info_json;
use crate::services::mods::trash;
use crate::services::scanner::watcher::WatcherState;
use serde::Serialize;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
pub struct BulkProgressPayload {
    pub label: String,
    pub current: usize,
    pub total: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkActionError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkResult {
    pub success: Vec<String>,
    pub failures: Vec<BulkActionError>,
}

pub async fn bulk_toggle(
    app: &AppHandle,
    pool: &SqlitePool,
    state: &WatcherState,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, String> {
    let total = paths.len();
    let action_label = if enable { "Enabling" } else { "Disabling" };
    let new_status = if enable { "ENABLED" } else { "DISABLED" };

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("{} {} mods...", action_label, total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut db_updates = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        let _ = app.emit(
            "bulk-progress",
            BulkProgressPayload {
                label: format!("{} {}/{}", action_label, i + 1, total),
                current: i + 1,
                total,
                active: true,
            },
        );

        match toggle_mod_inner(state, path.clone(), enable).await {
            Ok(new_path) => {
                db_updates.push((path.clone(), new_path.clone(), new_status.to_string()));
                success.push(new_path);
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    if !db_updates.is_empty() {
        if let Err(e) = mod_repo::batch_update_path_and_status(pool, &db_updates).await {
            log::error!("Failed batch updating mod paths: {}", e);
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_toggle_inner(
    state: &WatcherState,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        match toggle_mod_inner(state, path.clone(), enable).await {
            Ok(new_path) => success.push(new_path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_delete(
    app: &AppHandle,
    pool: &SqlitePool,
    state: &WatcherState,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let trash_dir = app_data_dir.join("trash");

    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir).map_err(|e| format!("Failed to create trash dir: {}", e))?;
    }

    let total = paths.len();
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("Deleting {} mods...", total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut db_deletes = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        let _ = app.emit(
            "bulk-progress",
            BulkProgressPayload {
                label: format!("Deleting {}/{}", i + 1, total),
                current: i + 1,
                total,
                active: true,
            },
        );

        match trash::move_to_trash_guarded(state, &trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => {
                db_deletes.push(path.clone());
                success.push(path.clone());
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    if !db_deletes.is_empty() {
        if let Err(e) = mod_repo::batch_delete_by_path(pool, &db_deletes).await {
            log::error!("Failed batch deleting mod paths from DB: {}", e);
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_delete_inner(
    state: &WatcherState,
    trash_dir: &Path,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        match trash::move_to_trash_guarded(state, trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_update_info(
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        match crate::commands::mods::mod_meta_cmds::update_mod_info(path.clone(), update.clone())
            .await
        {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_toggle_favorite(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<BulkResult, String> {
    let failures = Vec::new();
    let mut relatives = Vec::new();

    let game_mod_path = crate::database::game_repo::get_mod_path(pool, &game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Game not found or has no mods_path".to_string())?;

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in &folder_paths {
        let rel_path = std::path::Path::new(folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(folder_path))
            .to_string_lossy()
            .to_string();

        relatives.push(rel_path);
    }

    if let Err(e) =
        crate::database::mod_repo::batch_set_favorite(pool, &game_id, &relatives, favorite).await
    {
        return Err(e.to_string());
    }

    for folder_path in &folder_paths {
        let full_path = std::path::Path::new(&folder_path);
        if full_path.exists() {
            let _ = info_json::update_info_json(
                full_path,
                &info_json::ModInfoUpdate {
                    is_favorite: Some(favorite),
                    ..Default::default()
                },
            );
        }
    }
    Ok(BulkResult {
        success: folder_paths,
        failures,
    })
}

pub async fn bulk_pin(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<BulkResult, String> {
    let failures = Vec::new();
    let mut relatives = Vec::new();

    let game_mod_path = crate::database::game_repo::get_mod_path(pool, &game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Game not found or has no mods_path".to_string())?;

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in &folder_paths {
        let rel_path = std::path::Path::new(folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(folder_path))
            .to_string_lossy()
            .to_string();

        relatives.push(rel_path);
    }

    if let Err(e) =
        crate::database::mod_repo::batch_set_pinned(pool, &game_id, &relatives, pin).await
    {
        return Err(e.to_string());
    }

    Ok(BulkResult {
        success: folder_paths,
        failures,
    })
}
