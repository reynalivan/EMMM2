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
        app,
        pool,
        config,
        disk_reconcile_state,
        watcher.suppressor.clone(),
        game_id.to_string(),
        DiskReconcileReason::InternalMutation,
        changed_paths,
        false,
    )
    .await
    .map_err(AppError::Internal)?;

    app.emit("disk_reconcile:result", result)
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[specta::specta]
#[tauri::command]
pub async fn repair_orphan_mods(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
) -> Result<usize, AppError> {
    let repaired = metadata::repair_orphan_mods(pool.inner(), &game_id)
        .await
        .map_err(|e| AppError::Internal(e))?;

    if repaired > 0 {
        let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
            &pool,
            &config,
            watcher.suppressor.clone(),
            &game_id,
            &[],
            false,
            true,
        )
        .await;
    }

    Ok(repaired)
}

#[specta::specta]
#[tauri::command]
pub async fn pin_mod(
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    pin: bool,
) -> Result<(), AppError> {
    Ok(metadata::toggle_pin(&config, pool.inner(), &watcher, &game_id, &folder_path, pin).await?)
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
        .map_err(|e| AppError::Internal(e))
}

#[specta::specta]
#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, AppError> {
    metadata::get_active_mod_conflicts(pool.inner(), &game_id)
        .await
        .map_err(|e| AppError::Internal(e))
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
