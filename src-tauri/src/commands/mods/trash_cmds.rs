use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::trash;
use crate::services::scanner::watcher::WatcherState;
use tauri::{AppHandle, State};

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
    // `game_id` is required: it names the mods root the path must sit inside.
    // Without it the delete used to skip containment entirely and trash any
    // absolute path the caller sent.
    let validated = validate_path(&config, &game_id, &path)?;
    let preflight_paths = [validated.to_string_lossy().to_string()];
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;

    let op_guard = op_lock.acquire().await?;
    let mut result = trash::delete_mod_service(&state, &validated).await?;
    drop(op_guard);

    // Convergence: reconcile the deleted root so DB matches disk even if a
    // manual sync step missed a case.
    let settlement = crate::services::disk_reconcile::emit::settle_committed_reconcile(
        crate::services::disk_reconcile::emit::run_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
            vec![path],
        )
        .await,
    );
    if let Some(reconcile) = settlement.reconcile {
        result
            .collection_impact
            .merge(reconcile.collection_reference_impact);
    }
    result.sync_warning = settlement.sync_warning;

    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub async fn open_recycle_bin() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg("shell:RecycleBinFolder")
            .spawn()
            .map_err(|error| AppError::Io(format!("Failed to open Recycle Bin: {error}")))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::Validation(
            "Opening the system Recycle Bin is only supported on Windows".to_string(),
        ))
    }
}

#[cfg(test)]
#[path = "tests/trash_cmds_tests.rs"]
mod tests;
