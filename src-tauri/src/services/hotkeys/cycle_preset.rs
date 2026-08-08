//! Preset-cycling executor: the async half of the preset hotkeys.
//!
//! Kept out of `manager.rs` because none of it touches `HotkeyManager`'s
//! internals — it is Tauri-state plumbing plus collection orchestration.

use crate::domain::errors::AppError;
use std::path::Path;

use tauri::Manager;

use crate::services::config::ConfigService;
use crate::services::keyviewer::generator::StatusFields;

use super::actions::{self, CycleDirection};
use super::HotkeyConfig;

/// Fetch a managed state value, naming it in the error so a missing
/// registration is diagnosable from the log line alone.
fn require<'a, T: Send + Sync + 'static>(
    app: &'a tauri::AppHandle,
    what: &str,
) -> Result<tauri::State<'a, T>, AppError> {
    app.try_state::<T>()
        .ok_or_else(|| AppError::Internal(format!("{what} not available")))
}

pub(super) async fn execute_cycle_preset(
    app: &tauri::AppHandle,
    direction: CycleDirection,
) -> Result<String, AppError> {
    let config_state = require::<ConfigService>(app, "ConfigService")?;
    let pool_state = require::<sqlx::SqlitePool>(app, "SqlitePool")?;
    let watcher_state =
        require::<crate::services::scanner::watcher::WatcherState>(app, "WatcherState")?;
    let op_lock =
        require::<crate::services::fs_utils::operation_lock::OperationLock>(app, "OperationLock")?;

    let settings = config_state.get_settings();
    let game = settings
        .active_game()
        .ok_or_else(|| AppError::Internal("No active game selected".to_string()))?;
    let game_id = game.id.as_str();
    let safe_mode_enabled = settings.safe_mode.enabled;

    let collections = crate::services::collection_service::list_collections(
        pool_state.inner(),
        game_id,
        crate::domain::corridor::Corridor::from_is_safe(safe_mode_enabled),
        None,
    )
    .await?;

    if collections.is_empty() {
        let status = StatusFields {
            safe_mode: safe_mode_enabled,
            preset_name: Some("No presets configured".to_string()),
            ..Default::default()
        };
        write_runtime_status(pool_state.inner(), game_id, &status, &settings.hotkeys).await?;
        return Ok("No presets available".to_string());
    }

    let preset_names: Vec<String> = collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect();
    let corridor =
        crate::repo::corridor_repo::get(pool_state.inner(), game_id, safe_mode_enabled).await?;

    let current_collection_id = corridor.and_then(|c| c.active_collection_id);

    let current_name = current_collection_id.and_then(|id| {
        collections
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.as_str())
    });
    let target_name = actions::resolve_next_preset(&preset_names, current_name, direction)
        .ok_or_else(|| AppError::Internal("No presets available".to_string()))?;

    let target = collections
        .iter()
        .find(|collection| collection.name == target_name)
        .ok_or_else(|| AppError::Internal(format!("Target preset '{target_name}' not found")))?;

    let _lock = op_lock.inner().acquire().await?;

    let apply_result = crate::services::collection_service::apply_collection(
        crate::services::collection_service::ApplyCollectionRequest {
            pool: pool_state.inner(),
            game_id,
            collection_id: &target.id,
            is_safe: safe_mode_enabled,
            mods_path: game.mod_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: true,
            settings: settings.clone(),
        },
    )
    .await?;

    let planner = actions::plan_cycle_preset(&target.name, safe_mode_enabled);

    write_runtime_status(
        pool_state.inner(),
        game_id,
        &planner.status,
        &settings.hotkeys,
    )
    .await?;

    let reload_key = super::reload::trigger_reload_fixes(&settings)?;

    Ok(format!(
        "{} (changed components: {}, reload: {})",
        planner.summary, apply_result.mods_enabled, reload_key
    ))
}

async fn write_runtime_status(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    status: &StatusFields,
    hotkey_config: &HotkeyConfig,
) -> Result<(), AppError> {
    let Some(mods_path) = crate::repo::game_repo::get_mod_path(pool, game_id).await? else {
        return Ok(());
    };

    let status_dir = Path::new(&mods_path).join(".emmm_data").join("status");
    crate::services::keyviewer::generator::write_status_file(&status_dir, status, hotkey_config)?;

    Ok(())
}
