use crate::services::config::{AppSettings, ConfigService};
use tauri::State;

#[specta::specta]
#[tauri::command]
pub async fn get_settings(state: State<'_, ConfigService>) -> Result<AppSettings, String> {
    Ok(state.get_settings())
}

#[specta::specta]
#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, ConfigService>,
) -> Result<(), String> {
    state.save_settings(settings)
}

#[specta::specta]
#[tauri::command]
pub async fn set_active_game(
    game_id: Option<String>,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
) -> Result<(), String> {
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
) -> Result<(), String> {
    state.set_auto_close_launcher(enabled)
}

#[specta::specta]
#[tauri::command]
pub async fn run_maintenance(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(u64, u64), String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::services::app::maintenance_service::run_maintenance_counts(pool.inner(), &app_data_dir)
        .await
}

#[specta::specta]
#[tauri::command]
pub async fn clear_old_thumbnails() -> Result<u64, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    let pruned = ThumbnailCache::clear_old_cache(30)?;
    Ok(pruned as u64)
}

#[specta::specta]
#[tauri::command]
pub async fn reset_pin_with_recovery_code(
    code: String,
    state: State<'_, ConfigService>,
) -> Result<bool, String> {
    state.reset_pin_with_recovery_code(&code)
}

#[cfg(test)]
#[path = "tests/settings_cmds_tests.rs"]
mod tests;
