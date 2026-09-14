//! Preset-cycling executor: the async half of the preset hotkeys.
//!
//! Kept out of `manager.rs` because none of it touches `HotkeyManager`'s
//! internals — it is Tauri-state plumbing plus collection orchestration.

use crate::shared::errors::AppError;

use tauri::Manager;

use crate::modules::settings::application::config::ConfigService;

use crate::shared::path_key::canonical_name_key;

/// Direction for cycling through presets. Kept with the only executor that
/// consumes it; there is no separate action-planning layer anymore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleDirection {
    Next,
    Previous,
}

/// Select the next preset in a stable alphabetical order with wrap-around.
/// A missing current preset starts from the first one.
pub fn resolve_next_preset(
    preset_names: &[String],
    current_preset_name: Option<&str>,
    direction: CycleDirection,
) -> Option<String> {
    let mut sorted = preset_names.to_vec();
    sorted.sort_by_cached_key(|name| canonical_name_key(name));
    if sorted.is_empty() {
        return None;
    }

    let current_index = current_preset_name.and_then(|name| {
        let target = canonical_name_key(name);
        sorted
            .iter()
            .position(|preset| canonical_name_key(preset) == target)
    });
    let next_index = match (current_index, direction) {
        (Some(index), CycleDirection::Next) => (index + 1) % sorted.len(),
        (Some(index), CycleDirection::Previous) => index.checked_sub(1).unwrap_or(sorted.len() - 1),
        (None, _) => 0,
    };
    Some(sorted[next_index].clone())
}

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
    let watcher_state = require::<
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >(app, "WatcherState")?;
    let op_lock = require::<crate::modules::mutation::coordinator::MutationCoordinator>(
        app,
        "MutationCoordinator",
    )?;

    let settings = config_state.get_settings();
    let game = settings
        .active_game()
        .ok_or_else(|| AppError::Internal("No active game selected".to_string()))?;
    let game_id = game.id.as_str();

    let collections = crate::modules::collections::application::collection::list_collections(
        pool_state.inner(),
        game_id,
    )
    .await?;

    if collections.is_empty() {
        let sync = crate::modules::system::application::app::post_apply::request_overlay_sync_for_game(
            pool_state.inner(),
            &config_state,
            game_id,
            crate::modules::system::application::app::post_apply::OverlaySyncCause::CollectionApplied,
        )
        .await?
        .ensure_success()?;
        return Ok(format!("No presets available (overlay: {:?})", sync.reload));
    }

    let preset_names: Vec<String> = collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect();
    let current_collection_id =
        crate::modules::collections::adapters::sqlite::runtime::get(pool_state.inner(), game_id)
            .await?
            .and_then(|runtime| runtime.active_collection_id);

    let current_name = current_collection_id.and_then(|id| {
        collections
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.as_str())
    });
    let target_name = resolve_next_preset(&preset_names, current_name, direction)
        .ok_or_else(|| AppError::Internal("No presets available".to_string()))?;

    let target = collections
        .iter()
        .find(|collection| collection.name == target_name)
        .ok_or_else(|| AppError::Internal(format!("Target preset '{target_name}' not found")))?;

    let preflight_paths =
        crate::modules::collections::application::collection::collection_preflight_scope_paths(
            pool_state.inner(),
            game_id,
            &target.id,
            &game.mod_path,
        )
        .await?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        app,
        pool_state.inner(),
        game_id,
        Some(&preflight_paths),
    )
    .await?;
    let disk_reconcile = require::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >(app, "DiskReconcileState")?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(game_id, op_lock.inner_lock())
        .await?;

    let apply_result =
        crate::modules::collections::application::collection::apply_collection_durable(
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
            op_lock.inner(),
        )
        .await?;

    drop(mutation_lease);
    crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
        app,
        pool_state.inner(),
        game_id,
    )
    .await?;

    Ok(format!(
        "Preset: {} (changed components: {}; overlay sync requested by the collection apply)",
        target.name, apply_result.mods_enabled
    ))
}
