use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::disk_reconcile::orchestrator::DiskReconcileState;
use crate::services::disk_reconcile::types::DiskReconcileReason;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::{info_json, metadata};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use tauri::Emitter;

async fn emit_internal_disk_reconcile(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    config: &ConfigService,
    disk_reconcile_state: &DiskReconcileState,
    watcher: &WatcherState,
    game_id: &str,
    changed_paths: Vec<String>,
) -> Result<(), AppError> {
    let result = crate::services::disk_reconcile::orchestrator::reconcile_disk_state(
        crate::services::disk_reconcile::orchestrator::DiskReconcileContext {
            pool,
            config,
            state: disk_reconcile_state,
            watcher_suppressor: watcher.suppressor.clone(),
        },
        crate::services::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
            game_id.to_string(),
            DiskReconcileReason::InternalMutation,
            changed_paths,
            false,
        ),
    )
    .await
    .map_err(AppError::Internal)?;

    app.emit("disk_reconcile:result", result)
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[specta::specta]
#[tauri::command]
pub async fn toggle_favorite(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    favorite: bool,
) -> Result<(), AppError> {
    Ok(metadata::toggle_favorite(
        &config,
        pool.inner(),
        &watcher,
        &game_id,
        &folder_path,
        favorite,
    )
    .await?)
}

#[specta::specta]
#[tauri::command]
pub async fn toggle_mod_safe(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    safe: bool,
) -> Result<(), AppError> {
    metadata::toggle_mod_safe(
        &config,
        pool.inner(),
        &watcher,
        &game_id,
        &folder_path,
        safe,
    )
    .await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn suggest_random_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    is_safe: bool,
) -> Result<Vec<metadata::RandomModProposal>, AppError> {
    metadata::suggest_random_mods(pool.inner(), &game_id, is_safe)
        .await
        .map_err(AppError::Internal)
}

#[specta::specta]
#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, AppError> {
    metadata::get_active_mod_conflicts(pool.inner(), &game_id)
        .await
        .map_err(AppError::Internal)
}

#[specta::specta]
#[tauri::command]
pub async fn read_mod_info(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Option<info_json::ModInfo>, AppError> {
    let path = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(crate::domain::errors::MetadataError::Security(e)))?;
    Ok(info_json::read_info_json(&path)?)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn update_mod_info(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: tauri::State<'_, DiskReconcileState>,
    state: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, AppError> {
    let path = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(crate::domain::errors::MetadataError::Security(e)))?;
    let changed_path = path.join("info.json").to_string_lossy().to_string();
    let _guard = SuppressionGuard::new(&state.suppressor);
    let info = info_json::update_info_json(&path, &update)?;
    emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &config,
        &disk_reconcile_state,
        &state,
        &game_id,
        vec![changed_path],
    )
    .await?;

    Ok(info)
}

#[specta::specta]
#[tauri::command]
pub async fn set_mod_category(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    category: String,
) -> Result<(), AppError> {
    metadata::set_mod_category(&config, &pool, &game_id, &folder_path, &category).await?;
    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        &pool,
        &config,
        state.suppressor.clone(),
        &game_id,
        &[],
        false,
        true,
    )
    .await;

    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn set_object_mods_category(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    game_id: String,
    object_id: String,
    category: String,
) -> Result<usize, AppError> {
    let updated =
        set_object_mods_category_inner(pool.inner(), &game_id, &object_id, &category).await?;

    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        &pool,
        &config,
        state.suppressor.clone(),
        &game_id,
        &[],
        false,
        true,
    )
    .await;

    Ok(updated)
}

async fn set_object_mods_category_inner(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
    category: &str,
) -> Result<usize, AppError> {
    let result = sqlx::query("UPDATE mods SET object_type = ? WHERE game_id = ? AND object_id = ?")
        .bind(category)
        .bind(game_id)
        .bind(object_id)
        .execute(pool)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(result.rows_affected() as usize)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn move_mod_to_object(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    op_lock: tauri::State<'_, OperationLock>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    target_object_id: String,
    status: Option<String>,
) -> Result<(), AppError> {
    crate::services::mods::organizer_ext::move_mod_to_object_service(
        &config,
        pool.inner(),
        &op_lock,
        &watcher,
        &game_id,
        &folder_path,
        &target_object_id,
        status.as_deref(),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/mod_meta_cmds_tests.rs"]
mod tests;
