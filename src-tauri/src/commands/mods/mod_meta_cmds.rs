use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::{info_json, metadata};
use crate::services::scanner::watcher::WatcherState;

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
        // Sync in-game overlay artifacts (Req-43)
        let _ = crate::services::app::post_apply::trigger_overlay_refresh(
            &pool,
            &config,
            watcher.suppressor.clone(),
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
    let result = metadata::toggle_mod_safe(
        &config,
        pool.inner(),
        &watcher,
        &game_id,
        &folder_path,
        safe,
    )
    .await?;

    // Sync in-game overlay artifacts (Req-43)
    let _ = crate::services::app::post_apply::trigger_overlay_refresh(
        &pool,
        &config,
        watcher.suppressor.clone(),
    )
    .await;

    Ok(result)
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
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, AppError> {
    let path = PathGuard::validate_path(&config, &game_id, &folder_path)
        .map_err(|e| AppError::Metadata(crate::domain::errors::MetadataError::Security(e)))?;
    let info = info_json::update_info_json(&path, &update)?;

    // Sync in-game overlay artifacts (Req-43)
    let _ = crate::services::app::post_apply::trigger_overlay_refresh(
        &pool,
        &config,
        state.suppressor.clone(),
    )
    .await;

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

    // Sync in-game overlay artifacts (Req-43)
    let _ = crate::services::app::post_apply::trigger_overlay_refresh(
        &pool,
        &config,
        state.suppressor.clone(),
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

    // Sync in-game overlay artifacts (Req-43)
    let _ = crate::services::app::post_apply::trigger_overlay_refresh(
        &pool,
        &config,
        watcher.suppressor.clone(),
    )
    .await;

    Ok(())
}

#[cfg(test)]
#[path = "tests/mod_meta_cmds_tests.rs"]
mod tests;
