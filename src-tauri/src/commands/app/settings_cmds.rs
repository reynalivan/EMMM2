use crate::domain::errors::AppError;
use crate::services::config::{AppSettings, ConfigService};
use tauri::State;

#[specta::specta]
#[tauri::command]
pub async fn get_settings(state: State<'_, ConfigService>) -> Result<AppSettings, AppError> {
    Ok(state.get_settings())
}

#[specta::specta]
#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, ConfigService>,
) -> Result<(), AppError> {
    state.save_settings(settings)
}

#[specta::specta]
#[tauri::command]
pub async fn set_active_game(
    game_id: Option<String>,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
) -> Result<(), AppError> {
    state.set_active_game(game_id.clone())?;
    if game_id.is_some() {
        let _ = crate::services::app::post_apply::trigger_overlay_refresh(
            pool.inner(),
            &state,
            watcher_state.suppressor.clone(),
        )
        .await;
    }
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn set_auto_close_launcher(
    enabled: bool,
    state: State<'_, ConfigService>,
) -> Result<(), AppError> {
    state.set_auto_close_launcher(enabled)
}

#[specta::specta]
#[tauri::command]
pub async fn run_maintenance(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(u64, u64), AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;
    crate::services::app::maintenance_service::run_maintenance_counts(pool.inner(), &app_data_dir)
        .await
        .map_err(AppError::Internal)
}

#[specta::specta]
#[tauri::command]
pub async fn clear_old_thumbnails() -> Result<u64, AppError> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    let pruned = ThumbnailCache::clear_old_cache(30).map_err(AppError::Internal)?;
    Ok(pruned as u64)
}

#[specta::specta]
#[tauri::command]
pub async fn reset_pin_with_recovery_code(
    code: String,
    state: State<'_, ConfigService>,
) -> Result<bool, AppError> {
    state.reset_pin_with_recovery_code(&code)
}

#[cfg(test)]
#[path = "tests/settings_cmds_tests.rs"]
mod tests;
