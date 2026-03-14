use crate::services::config::{pin_guard::PinVerifyStatus, AppSettings, ConfigService};
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, ConfigService>) -> Result<AppSettings, String> {
    Ok(state.get_settings())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, ConfigService>,
) -> Result<(), String> {
    state.save_settings(settings)
}

#[tauri::command]
pub async fn set_safe_mode_pin(pin: String, state: State<'_, ConfigService>) -> Result<(), String> {
    state.set_pin(&pin)
}

#[tauri::command]
pub async fn verify_pin(
    pin: String,
    state: State<'_, ConfigService>,
) -> Result<PinVerifyStatus, String> {
    Ok(state.verify_pin_status(&pin))
}

#[tauri::command]
pub async fn set_active_game(
    game_id: Option<String>,
    state: State<'_, ConfigService>,
) -> Result<(), String> {
    state.set_active_game(game_id)
}

#[tauri::command]
pub async fn set_safe_mode_enabled(
    enabled: bool,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<(), String> {
    let _lock = op_lock.acquire().await?;

    // Phase 22: Strict Privacy Boundaries (Mutually Exclusive Corridor)
    // Cannot toggle privacy mode if there are active mods for the current game
    let settings = state.get_settings();
    if let Some(game_id) = settings.active_game_id {
        let active_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM mods WHERE game_id = ? AND status = 'enabled'",
        )
        .bind(&game_id)
        .fetch_one(pool.inner())
        .await
        .unwrap_or(0);

        if active_count > 0 {
            return Err("Failed to switch Privacy Mode: You have active mods. Please disable all mods before switching modes.".to_string());
        }
    }

    let mode = if enabled {
        crate::services::privacy::Mode::SFW
    } else {
        crate::services::privacy::Mode::NSFW
    };
    crate::services::privacy::PrivacyManager::switch_mode(mode, &pool, &watcher_state).await?;
    state.set_safe_mode_enabled(enabled)
}

#[tauri::command]
pub async fn set_auto_close_launcher(
    enabled: bool,
    state: State<'_, ConfigService>,
) -> Result<(), String> {
    state.set_auto_close_launcher(enabled)
}

#[tauri::command]
pub async fn run_maintenance(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<String, String> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::services::app::maintenance_service::run_maintenance(pool.inner(), &app_data_dir).await
}

#[tauri::command]
pub async fn clear_old_thumbnails() -> Result<String, String> {
    use crate::services::images::thumbnail_cache::ThumbnailCache;
    let pruned = ThumbnailCache::clear_old_cache(30)?;
    Ok(format!("Cleared {} old thumbnails.", pruned))
}

#[cfg(test)]
#[path = "tests/settings_cmds_tests.rs"]
mod tests;
