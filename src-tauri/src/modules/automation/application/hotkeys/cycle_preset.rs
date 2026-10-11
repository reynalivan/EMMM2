//! Preset-cycling executor: the async half of the preset hotkeys.
//!
//! Kept out of `manager.rs` because none of it touches `HotkeyManager`'s
//! internals — it is Tauri-state plumbing plus collection orchestration.

use crate::shared::errors::AppError;

use tauri::Manager;

use super::manager::HotkeyGameContext;
use super::notice::{HotkeyActionOutcome, HotkeyNoticeKind};
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
    context: &HotkeyGameContext,
) -> Result<HotkeyActionOutcome, AppError> {
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
    context.validate(&settings)?;
    let game_id = context.game_id();
    let mods_path = context.mods_path();
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        game_id,
        crate::modules::mutation::api::ImmutableMutationKind::PresetApply,
    )?;

    let collections = crate::modules::collections::application::collection::list_collections(
        pool_state.inner(),
        game_id,
    )
    .await?;

    let runtime_before_preflight =
        crate::modules::collections::adapters::sqlite::runtime::get(pool_state.inner(), game_id)
            .await?
            .and_then(|runtime| runtime.active_collection_id);
    let previous_before_preflight = runtime_before_preflight.as_deref().and_then(|id| {
        collections
            .iter()
            .find(|collection| collection.id == id)
            .map(|collection| collection.name.clone())
    });

    if collections.is_empty() {
        let generation = crate::modules::reconciliation::api::enqueue_runtime_sync(
            app,
            pool_state.inner(),
            game_id,
            crate::modules::reconciliation::api::RuntimeSyncCause::CollectionApplied,
        );
        return Ok(HotkeyActionOutcome {
            kind: HotkeyNoticeKind::NoPresets,
            previous_collection: previous_before_preflight.clone(),
            collection: previous_before_preflight,
            safe_mode: settings.safety.runtime_safe_mode_for(game_id),
            message: Some("No presets are available for this game".to_string()),
            runtime_generation: Some(generation),
            log_summary: format!(
                "No presets available (overlay queued as generation {generation})"
            ),
        });
    }

    let preset_names: Vec<String> = collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect();
    let current_name = runtime_before_preflight.as_deref().and_then(|id| {
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
            mods_path,
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
        .acquire_ready_mutation_lease(game_id, op_lock.inner_lock())
        .await?;
    context.validate(&config_state.get_settings())?;
    crate::modules::reconciliation::api::ensure_projection_epoch(
        config_state.inner(),
        game_id,
        context.root_identity(),
    )?;

    let settings = config_state.get_settings();
    context.validate(&settings)?;

    // The preflight target was chosen before waiting for mutation authority.
    // Reject rather than applying a target derived from a stale active preset.
    let authoritative_collection_id =
        crate::modules::collections::adapters::sqlite::runtime::get(pool_state.inner(), game_id)
            .await?
            .and_then(|runtime| runtime.active_collection_id);
    if authoritative_collection_id != runtime_before_preflight {
        return Err(AppError::Validation(
            "Collection state changed while the hotkey was waiting; retry the hotkey".to_string(),
        ));
    }
    let previous_collection = authoritative_collection_id.as_deref().and_then(|id| {
        collections
            .iter()
            .find(|collection| collection.id == id)
            .map(|collection| collection.name.clone())
    });
    let safe_mode = settings.safety.runtime_safe_mode_for(game_id);

    let apply_result =
        crate::modules::collections::application::collection::apply_collection_durable(
            crate::modules::collections::application::collection::ApplyCollectionRequest {
                pool: pool_state.inner(),
                game_id,
                collection_id: &target.id,
                capture_last_changes: true,
                mods_path: mods_path.to_path_buf(),
                suppressor: watcher_state.suppressor.clone(),
                ignore_missing: true,
                settings: settings.clone(),
            },
            op_lock.inner(),
        )
        .await?;

    let generation = crate::modules::reconciliation::api::enqueue_runtime_sync_for_rewrites(
        app,
        pool_state.inner(),
        game_id,
        mods_path,
        crate::modules::reconciliation::api::RuntimeSyncCause::CollectionApplied,
        &apply_result.runtime_path_rewrites,
    );
    drop(mutation_lease);

    Ok(HotkeyActionOutcome {
        kind: HotkeyNoticeKind::CollectionApplied,
        previous_collection,
        collection: Some(target.name.clone()),
        safe_mode,
        message: None,
        runtime_generation: Some(generation),
        log_summary: format!(
            "Preset: {} (changed components: {}; overlay queued as generation {generation})",
            target.name, apply_result.mods_enabled,
        ),
    })
}
