use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
use crate::commands::mods::trash_cmds::delete_mod_inner;
use crate::services::core::operation_lock::OperationLock;
use crate::services::mod_files::info_json;
use crate::services::scanner::watcher::WatcherState;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};

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

#[tauri::command]
pub async fn bulk_toggle_mods(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;
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

        match toggle_mod_inner(&state, path.clone(), enable).await {
            Ok(new_path) => {
                let _ = sqlx::query(
                    "UPDATE mods SET folder_path = ?, status = ? WHERE folder_path = ?",
                )
                .bind(&new_path)
                .bind(new_status)
                .bind(path)
                .execute(&*pool)
                .await;
                success.push(new_path);
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_toggle_mods_inner(
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

#[tauri::command]
pub async fn bulk_delete_mods(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;
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

        match delete_mod_inner(&state, &trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => {
                // Sync DB: remove mod record since folder is trashed
                let _ = sqlx::query("DELETE FROM mods WHERE folder_path = ?")
                    .bind(path)
                    .execute(&*pool)
                    .await;
                success.push(path.clone());
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_delete_mods_inner(
    state: &WatcherState,
    trash_dir: &Path,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, String> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        match delete_mod_inner(state, trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError { path, error: e }),
        }
    }
    Ok(BulkResult { success, failures })
}

#[tauri::command]
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

#[tauri::command]
pub async fn bulk_toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<BulkResult, String> {
    use sqlx::Row;
    let mut success = Vec::new();
    let mut failures = Vec::new();

    let game_mod_path: String = match sqlx::query("SELECT mod_path FROM games WHERE id = ?")
        .bind(&game_id)
        .fetch_optional(pool.inner())
        .await
    {
        Ok(Some(gr)) => gr.try_get("mod_path").unwrap_or_default(),
        _ => return Err("Game not found or has no mods_path".to_string()),
    };

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in folder_paths {
        let rel_path = std::path::Path::new(&folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(&folder_path))
            .to_string_lossy()
            .to_string();

        if let Err(e) = sqlx::query("UPDATE mods SET is_favorite = ? WHERE folder_path = ? AND game_id = ?")
            .bind(favorite)
            .bind(&rel_path)
            .bind(&game_id)
            .execute(pool.inner())
            .await
        {
            failures.push(BulkActionError {
                path: folder_path.clone(),
                error: e.to_string(),
            });
            continue;
        }

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
        success.push(folder_path);
    }
    Ok(BulkResult { success, failures })
}

#[tauri::command]
pub async fn bulk_pin_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<BulkResult, String> {
    use sqlx::Row;
    let mut success = Vec::new();
    let mut failures = Vec::new();

    let game_mod_path: String = match sqlx::query("SELECT mod_path FROM games WHERE id = ?")
        .bind(&game_id)
        .fetch_optional(pool.inner())
        .await
    {
        Ok(Some(gr)) => gr.try_get("mod_path").unwrap_or_default(),
        _ => return Err("Game not found or has no mods_path".to_string()),
    };

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in folder_paths {
        let rel_path = std::path::Path::new(&folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(&folder_path))
            .to_string_lossy()
            .to_string();

        match sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path = ? AND game_id = ?")
            .bind(pin)
            .bind(&rel_path)
            .bind(&game_id)
            .execute(pool.inner())
            .await
        {
            Ok(_) => success.push(folder_path),
            Err(e) => failures.push(BulkActionError {
                path: folder_path,
                error: e.to_string(),
            }),
        }
    }
    Ok(BulkResult { success, failures })
}
