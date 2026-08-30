//! Preset-cycling executor: the async half of the preset hotkeys.
//!
//! Kept out of `manager.rs` because none of it touches `HotkeyManager`'s
//! internals — it is Tauri-state plumbing plus collection orchestration.

use crate::shared::errors::AppError;
use std::path::Path;

use tauri::Manager;

use crate::modules::settings::application::config::ConfigService;
use crate::modules::automation::application::keyviewer::generator::StatusFields;

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
        require::<crate::modules::workspace::application::scanner::watcher::WatcherState>(app, "WatcherState")?;
    let op_lock =
        require::<crate::platform::fs::operation_lock::OperationLock>(app, "OperationLock")?;

    let settings = config_state.get_settings();
    let game = settings
        .active_game()
        .ok_or_else(|| AppError::Internal("No active game selected".to_string()))?;
    let game_id = game.id.as_str();

    let collections =
        crate::modules::collections::application::collection::list_collections(pool_state.inner(), game_id).await?;

    if collections.is_empty() {
        let status = StatusFields {
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
    let current_collection_id =
        crate::modules::collections::adapters::outbound::sqlite::runtime::get(pool_state.inner(), game_id)
            .await?
            .and_then(|runtime| runtime.active_collection_id);

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

    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(
        app,
        pool_state.inner(),
        game_id,
    )
    .await?;
    let disk_reconcile = require::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >(app, "DiskReconcileState")?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(game_id, op_lock.inner())
        .await?;

    let apply_result = crate::modules::collections::application::collection::apply_collection(
        crate::modules::collections::application::collection::ApplyCollectionRequest {
            pool: pool_state.inner(),
            game_id,
            collection_id: &target.id,
            capture_last_changes: true,
            mods_path: game.mod_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: true,
            settings: settings.clone(),
        },
    )
    .await?;

    drop(mutation_lease);
    crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
        app,
        pool_state.inner(),
        game_id,
    )
    .await?;

    let planner = actions::plan_cycle_preset(&target.name);

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
    let Some(mods_path) = crate::modules::games::adapters::outbound::sqlite::game::get_mod_path(pool, game_id).await? else {
        return Ok(());
    };

    let status_dir = Path::new(&mods_path).join(".emmm_data").join("status");
    crate::modules::automation::application::keyviewer::generator::write_status_file(&status_dir, status, hotkey_config)?;

    Ok(())
}
