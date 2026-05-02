use crate::repo::mod_repo;
use crate::services::mods::core_ops::toggle_mod_inner;
use crate::services::mods::info_json;
use crate::services::mods::trash;
use crate::services::scanner::watcher::WatcherState;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkProgressPayload {
    pub label: String,
    #[specta(type = f64)]
    pub current: usize,
    #[specta(type = f64)]
    pub total: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkActionError {
    pub path: String,
    pub error: crate::domain::errors::AppError,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkResult {
    pub success: Vec<String>,
    pub failures: Vec<BulkActionError>,
}

/// Bulk toggle mods on disk and sync DB.
///
/// `mods_path` must be provided and already validated by the caller (command layer).
/// Paths in `paths` are absolute; DB updates use relative paths computed from `mods_path`.
pub async fn bulk_toggle(
    app: &AppHandle,
    config: &crate::services::config::ConfigService,
    pool: &SqlitePool,
    state: &WatcherState,
    mods_path: &str,
    game_id: &str,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let total = paths.len();
    let action_label = if enable { "Enabling" } else { "Disabling" };
    let new_status_enum = if enable {
        crate::database::models::ItemStatus::Enabled
    } else {
        crate::database::models::ItemStatus::Disabled
    };

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
    // (old_abs, new_abs, ItemStatus) — for DB batch update
    let mut db_updates = Vec::new();

    // Opt-O: Batch progress — emit every N items to reduce IPC overhead
    let progress_interval = std::cmp::max(1, total / 10);

    for (i, path) in paths.iter().enumerate() {
        if i % progress_interval == 0 || i == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: format!("{} {}/{}", action_label, i + 1, total),
                    current: i + 1,
                    total,
                    active: true,
                },
            );
        }

        match toggle_mod_inner(state, path.clone(), enable).await {
            Ok(new_abs_path) => {
                // Convert absolute paths to relative for DB storage
                let old_rel = Path::new(path)
                    .strip_prefix(mods_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.clone());
                let new_rel = Path::new(&new_abs_path)
                    .strip_prefix(mods_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| new_abs_path.clone());

                db_updates.push((old_rel.clone(), new_rel.clone(), new_status_enum));
                success.push(new_abs_path.clone());

                // Collection auto-healing: cascade path rename to all saved collections
                if old_rel != new_rel {
                    let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
                        pool, &old_rel, &new_rel, None,
                    )
                    .await;
                }
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    if !db_updates.is_empty() {
        if let Err(e) = mod_repo::batch_update_path_and_status(pool, &db_updates).await {
            log::error!("Failed batch updating mod paths after bulk toggle: {}", e);
        }

        let _ = crate::services::runtime_projection_service::rebuild_game_projection(pool, game_id)
            .await;
    }

    // Recompute corridor signatures so dirty detection stays in sync
    let _ = crate::services::corridor_service::recompute_signature(pool, game_id, true).await;
    let _ = crate::services::corridor_service::recompute_signature(pool, game_id, false).await;

    // Trigger Dirty State: Register unsaved changes for the affected corridors
    if !db_updates.is_empty() {
        // Collect subset of relative paths to check which corridors are affected
        let rel_paths: Vec<String> = db_updates
            .iter()
            .map(|(_, new_rel, _)| new_rel.clone())
            .collect();
        let affected_corridors: Vec<i32> = sqlx::query_scalar(
            "SELECT DISTINCT is_safe FROM mods WHERE game_id = ? AND folder_path IN (SELECT value FROM json_each(?))"
        )
        .bind(game_id)
        .bind(serde_json::to_string(&rel_paths).unwrap_or_else(|_| "[]".to_string()))
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let safe_contexts = affected_corridors
            .into_iter()
            .map(|safe_value| safe_value != 0)
            .collect::<Vec<_>>();
        let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
            pool,
            config,
            state.suppressor.clone(),
            game_id,
            &safe_contexts,
            true,
            true,
        )
        .await;
    }

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: "Done".to_string(),
            current: total,
            total,
            active: false,
        },
    );

    Ok(BulkResult { success, failures })
}

/// Internal bulk toggle without progress events (for test use only).
pub async fn bulk_toggle_inner(
    state: &WatcherState,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
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
    config: &crate::services::config::ConfigService,
    pool: &SqlitePool,
    state: &WatcherState,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        crate::domain::errors::AppError::Io(format!("Failed to get app data dir: {}", e))
    })?;
    let trash_dir = app_data_dir.join("trash");

    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir).map_err(|e| {
            crate::domain::errors::AppError::Io(format!("Failed to create trash dir: {}", e))
        })?;
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

    // Opt-O: Batch progress — emit every N items
    let progress_interval = std::cmp::max(1, total / 10);

    for (i, path) in paths.iter().enumerate() {
        if i % progress_interval == 0 || i == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: format!("Deleting {}/{}", i + 1, total),
                    current: i + 1,
                    total,
                    active: true,
                },
            );
        }

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
        // Detect which corridors were affected BEFORE deleting from DB or after if we still have the paths
        // Actually, we should check corridor safety BEFORE we commit the delete to DB.
        // But we already moved them on disk. Let's query based on the paths we're about to delete.
        let affected_corridors: Vec<i32> = if let Some(gid) = &game_id {
            // Get mod path to compute relative paths
            let mp = crate::repo::game_repo::get_mod_path(pool, gid)
                .await
                .ok()
                .flatten();
            if let Some(base_path) = mp {
                let base = Path::new(&base_path);
                let relatives: Vec<String> = db_deletes
                    .iter()
                    .map(|p| {
                        Path::new(p)
                            .strip_prefix(base)
                            .map(|sp| sp.to_string_lossy().to_string())
                            .unwrap_or_else(|_| p.clone())
                    })
                    .collect();

                sqlx::query_scalar(
                    "SELECT DISTINCT is_safe FROM mods WHERE game_id = ? AND folder_path IN (SELECT value FROM json_each(?))"
                )
                .bind(gid)
                .bind(serde_json::to_string(&relatives).unwrap_or_else(|_| "[]".to_string()))
                .fetch_all(pool)
                .await
                .unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if let Err(e) = mod_repo::batch_delete_by_path(pool, &db_deletes).await {
            log::error!("Failed batch deleting mod paths from DB: {}", e);
        }

        // Trigger dirty state for cada affected corridor
        if let Some(gid) = game_id {
            let _ =
                crate::services::runtime_projection_service::rebuild_game_projection(pool, &gid)
                    .await;
            let safe_contexts = affected_corridors
                .into_iter()
                .map(|safe_value| safe_value != 0)
                .collect::<Vec<_>>();
            let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
                pool,
                config,
                state.suppressor.clone(),
                &gid,
                &safe_contexts,
                true,
                true,
            )
            .await;
        }
    }

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: "Done".to_string(),
            current: total,
            total,
            active: false,
        },
    );

    Ok(BulkResult { success, failures })
}

/// Internal bulk delete without progress events (for test use only).
pub async fn bulk_delete_inner(
    state: &WatcherState,
    trash_dir: &Path,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, crate::domain::errors::AppError> {
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
    config: &crate::services::config::ConfigService,
    game_id: &str,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        let canonical =
            crate::services::fs_utils::guard::PathGuard::validate_path(config, game_id, &path)
                .map_err(crate::domain::errors::AppError::Security)?;

        match info_json::update_info_json(&canonical, &update) {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError {
                path,
                error: crate::domain::errors::AppError::Metadata(e),
            }),
        }
    }
    Ok(BulkResult { success, failures })
}

pub async fn bulk_toggle_favorite(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let failures = Vec::new();
    let mut relatives = Vec::new();

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, &game_id)
        .await?
        .ok_or_else(|| {
            crate::domain::errors::AppError::NotFound(
                "Game not found or has no mods_path".to_string(),
            )
        })?;

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
        crate::repo::mod_repo::batch_set_favorite(pool, &game_id, &relatives, favorite).await
    {
        return Err(crate::domain::errors::AppError::Io(e.to_string()));
    }

    // Opt-R: Parallel info.json writes using rayon
    use rayon::prelude::*;
    let update_for_parallel = info_json::ModInfoUpdate {
        is_favorite: Some(favorite),
        ..Default::default()
    };
    folder_paths.par_iter().for_each(|folder_path| {
        let full_path = std::path::Path::new(folder_path);
        if full_path.exists() {
            let _ = info_json::update_info_json(full_path, &update_for_parallel);
        }
    });
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
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let failures = Vec::new();
    let mut relatives = Vec::new();

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, &game_id)
        .await?
        .ok_or_else(|| {
            crate::domain::errors::AppError::NotFound(
                "Game not found or has no mods_path".to_string(),
            )
        })?;

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in &folder_paths {
        let rel_path = std::path::Path::new(folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(folder_path))
            .to_string_lossy()
            .to_string();

        relatives.push(rel_path);
    }

    if let Err(e) = crate::repo::mod_repo::batch_set_pinned(pool, &game_id, &relatives, pin).await {
        return Err(crate::domain::errors::AppError::Io(e.to_string()));
    }

    Ok(BulkResult {
        success: folder_paths,
        failures,
    })
}
