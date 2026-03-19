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

/// Sets a new PIN and returns a one-time recovery code (e.g. `EMMM-4F2A-9B87-CC1E`).
/// The recovery code is shown once; a SHA-256 hash is stored in settings.
#[tauri::command]
pub async fn set_safe_mode_pin_with_recovery(
    pin: String,
    state: State<'_, ConfigService>,
) -> Result<String, String> {
    state.set_pin_with_recovery(&pin)
}

/// Resets the PIN using a recovery code. Returns `true` if the code was valid.
#[tauri::command]
pub async fn reset_pin_with_recovery_code(
    code: String,
    state: State<'_, ConfigService>,
) -> Result<bool, String> {
    state.reset_pin_with_recovery_code(&code)
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
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<(), String> {
    let _lock = op_lock.acquire().await?;
    state.set_active_game(game_id.clone())?;
    if game_id.is_none() {
        return Ok(());
    }

    crate::services::corridor_runtime::reconcile_active_game_corridor(
        pool.inner(),
        &watcher_state,
        state.inner(),
    )
    .await?;

    crate::services::collections::materialize_game_collections_if_missing(
        pool.inner(),
        game_id.as_deref().ok_or("No active game selected")?,
    )
    .await?;

    Ok(())
}

#[tauri::command]
pub async fn set_safe_mode_enabled(
    enabled: bool,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher_state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<crate::services::privacy::CorridorSwitchResult, String> {
    let _lock = op_lock.acquire().await?;

    let settings = state.get_settings();
    let game_id = settings.active_game_id.ok_or("No active game selected")?;

    let mode = if enabled {
        crate::services::privacy::Mode::SFW
    } else {
        crate::services::privacy::Mode::NSFW
    };
    let result =
        crate::services::privacy::switch_mode(mode, &pool, &watcher_state, &game_id).await?;
    state.set_safe_mode_enabled(enabled)?;
    let _ = crate::services::corridor_runtime::reconcile_active_game_corridor(
        pool.inner(),
        &watcher_state,
        state.inner(),
    )
    .await?;
    let _ = crate::services::collections::materialize_game_collections_if_missing(
        pool.inner(),
        &game_id,
    )
    .await?;
    Ok(result)
}

#[tauri::command]
pub async fn preview_corridor_switch(
    target_enabled: bool,
    state: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<crate::services::corridor_types::CorridorPreview, String> {
    let settings = state.get_settings();
    let game_id = settings.active_game_id.ok_or("No active game selected")?;
    let current_safe = settings.safe_mode.enabled;

    crate::services::corridor_runtime::preview_corridor_switch(
        &pool,
        &game_id,
        current_safe,
        target_enabled,
    )
    .await
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
