use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::trash;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use tauri::{AppHandle, Manager, State};

#[specta::specta]
#[tauri::command]
pub async fn delete_mod(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    path: String,
    game_id: String,
) -> Result<trash::DeleteModResult, AppError> {
    let trash_dir = trash::trash_dir(&app)?;

    // `game_id` is required: it names the mods root the path must sit inside.
    // Without it the delete used to skip containment entirely and trash any
    // absolute path the caller sent.
    let validated = validate_path(&config, &game_id, &path)?;

    let op_guard = op_lock.acquire().await?;
    let result = trash::delete_mod_service(
        &config, &pool, &state, &op_guard, trash_dir, &validated, &game_id,
    )
    .await?;

    // Convergence: reconcile the deleted root so DB matches disk even if a
    // manual sync step missed a case.
    if let Err(error) = emit_internal_disk_reconcile(&app, pool.inner(), &game_id, vec![path]).await
    {
        log::warn!("Post-delete disk reconcile failed: {error}");
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn restore_mod(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    trash_id: String,
    game_id: Option<String>,
) -> Result<String, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("Failed to get app data dir: {e}")))?;

    let result = {
        let _guard = SuppressionGuard::new(&state.suppressor);
        trash::restore_from_trash(&trash_id, &app_data_dir.join("trash"), game_id.as_ref())?
    };

    // Single-writer: events were suppressed during the restore, so the scoped
    // reconcile is what re-creates the row. With no explicit game id, resolve
    // it from the restored path's mods root.
    let config = {
        use tauri::Manager;
        app.state::<crate::services::config::ConfigService>()
    };
    let reconcile_game_id = game_id
        .clone()
        .or_else(|| config.game_id_for_path(std::path::Path::new(&result)));
    if let Some(game_id) = &reconcile_game_id {
        if let Err(error) =
            emit_internal_disk_reconcile(&app, pool.inner(), game_id, vec![result.clone()]).await
        {
            log::warn!("Post-restore disk reconcile failed: {error}");
        }
    } else {
        log::warn!(
            "Restored path not under any configured mods root; skipping reconcile: {result}"
        );
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn list_trash(app: AppHandle) -> Result<Vec<trash::TrashMetadata>, AppError> {
    trash::list_trash(&trash::trash_dir(&app)?)
}

#[specta::specta]
#[tauri::command]
pub async fn empty_trash(app: AppHandle) -> Result<u64, AppError> {
    trash::empty_trash(&trash::trash_dir(&app)?)
}

#[cfg(test)]
#[path = "tests/trash_cmds_tests.rs"]
mod tests;
