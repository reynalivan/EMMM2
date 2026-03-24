use crate::domain::errors::AppError;
use crate::repo::game_repo;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::bulk;
use crate::services::mods::info_json;
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, State};

#[specta::specta]
#[tauri::command]
pub async fn bulk_toggle_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, AppError> {
    let mods_path = game_repo::get_mod_path(pool.inner(), &game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods path".to_string()))?;

    // Security validation for all paths
    for p in &paths {
        PathGuard::validate_path(&config, &game_id, p).map_err(AppError::Security)?;
    }

    let _lock = op_lock.acquire().await.map_err(AppError::Io)?;
    let result = bulk::bulk_toggle(
        &app,
        pool.inner(),
        &state,
        &mods_path,
        &game_id,
        paths,
        enable,
    )
    .await?;

    let post_ctx = crate::services::app::post_apply::PostApplyContext {
        game_id,
        pool: pool.inner().clone(),
        is_safe: config.get_settings().safe_mode.enabled,
        mods_path: mods_path.into(),
        suppressor: state.suppressor.clone(),
        settings: config.get_settings(),
        status_fields: None,
    };
    let _ = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await;

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_delete_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: Option<String>,
    paths: Vec<String>,
) -> Result<bulk::BulkResult, AppError> {
    if let Some(ref gid) = game_id {
        for p in &paths {
            PathGuard::validate_path(&config, gid, p).map_err(AppError::Security)?;
        }
    }

    let _lock = op_lock.acquire().await.map_err(AppError::Io)?;
    let result = bulk::bulk_delete(&app, pool.inner(), &state, paths, game_id.clone()).await?;

    if let Some(gid) = game_id {
        if let Some(mods_path) = game_repo::get_mod_path(pool.inner(), &gid).await? {
            let post_ctx = crate::services::app::post_apply::PostApplyContext {
                game_id: gid,
                pool: pool.inner().clone(),
                is_safe: config.get_settings().safe_mode.enabled,
                mods_path: mods_path.into(),
                suppressor: state.suppressor.clone(),
                settings: config.get_settings(),
                status_fields: None,
            };
            let _ = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await;
        }
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_update_info(
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, AppError> {
    let result = bulk::bulk_update_info(&config, &game_id, paths, update).await?;

    if let Some(mods_path) = game_repo::get_mod_path(pool.inner(), &game_id).await? {
        let post_ctx = crate::services::app::post_apply::PostApplyContext {
            game_id: game_id.clone(),
            pool: pool.inner().clone(),
            is_safe: config.get_settings().safe_mode.enabled,
            mods_path: mods_path.into(),
            suppressor: state.suppressor.clone(),
            settings: config.get_settings(),
            status_fields: None,
        };
        let _ = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await;
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, AppError> {
    bulk::bulk_toggle_favorite(&pool, game_id, folder_paths, favorite).await
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_pin_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, AppError> {
    bulk::bulk_pin(&pool, game_id, folder_paths, pin).await
}

#[cfg(test)]
#[path = "tests/mod_bulk_cmds_tests.rs"]
mod tests;
