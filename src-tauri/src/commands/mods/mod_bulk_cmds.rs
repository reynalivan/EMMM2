use crate::domain::errors::AppError;
use crate::repo::game_repo;
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::bulk;
use crate::services::mods::info_json;
use crate::services::scanner::watcher::WatcherState;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, State};

/// Cooperative cancel for the two bulk actions that walk the filesystem one
/// folder at a time. A single flag is enough: `OperationLock` already
/// serializes bulk runs, so two batches are never in flight together.
#[derive(Default)]
pub struct BulkCancelState(AtomicBool);

impl BulkCancelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears a cancel left over from an earlier batch and hands back the flag.
    /// Callers already hold the operation lock, so this cannot wipe a cancel
    /// aimed at a run that is still going.
    fn begin(&self) -> &AtomicBool {
        self.0.store(false, Ordering::SeqCst);
        &self.0
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Stop the running bulk toggle/delete after the item in flight. Work already
/// done stays done — the trailing reconcile still converges the DB.
#[specta::specta]
#[tauri::command]
pub async fn bulk_cancel(cancel_state: State<'_, BulkCancelState>) -> Result<(), AppError> {
    cancel_state.cancel();
    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_toggle_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, AppError> {
    // Security validation for all paths
    crate::services::fs_utils::guard::validate_paths(&config, &game_id, &paths)?;

    let _lock = op_lock.acquire().await?;
    let result = bulk::bulk_toggle(
        &app,
        pool.inner(),
        &state,
        &game_id,
        paths,
        enable,
        cancel_state.begin(),
    )
    .await?;

    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_delete_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
) -> Result<bulk::BulkResult, AppError> {
    // Required, like `delete_mod`: it names the mods root the paths must sit
    // inside, and the game whose index rows may be pruned. Optional, it let a
    // caller skip containment entirely.
    crate::services::fs_utils::guard::validate_paths(&config, &game_id, &paths)?;

    let _lock = op_lock.acquire().await?;
    let result = bulk::bulk_delete(
        &app,
        &config,
        pool.inner(),
        &state,
        paths,
        &game_id,
        cancel_state.begin(),
    )
    .await?;

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_update_info(
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, AppError> {
    let _lock = op_lock.acquire().await?;
    let validated = crate::services::fs_utils::guard::validate_paths(&config, &game_id, &paths)?;
    let mut result = bulk::bulk_update_info(&validated, update).await?;

    if let Some(mods_path) = game_repo::get_mod_path(pool.inner(), &game_id).await? {
        let post_ctx = crate::services::app::post_apply::PostApplyContext {
            game_id: game_id.clone(),
            pool: pool.inner().clone(),
            is_safe: config.get_settings().safe_mode.enabled,
            mods_path: mods_path.into(),
            hotkeys: config.with_settings(|settings| settings.hotkeys.clone()),
            status_fields: None,
        };
        if let Err(error) = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await {
            result.failures.push(bulk::BulkActionError {
                path: ".emmm_data".to_string(),
                error,
            });
        }
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_toggle_favorite(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, AppError> {
    // Writes info.json inside folders a concurrent toggle/delete may be renaming.
    let _lock = op_lock.acquire().await?;
    bulk::bulk_toggle_favorite(&pool, game_id, folder_paths, favorite).await
}

#[specta::specta]
#[tauri::command]
pub async fn bulk_pin_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, AppError> {
    // Paths are resolved against the mods root a concurrent toggle/delete may be moving.
    let _lock = op_lock.acquire().await?;
    bulk::bulk_pin(&pool, game_id, folder_paths, pin).await
}
