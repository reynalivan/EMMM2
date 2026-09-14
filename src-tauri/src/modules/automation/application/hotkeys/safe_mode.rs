//! Per-game Safe Mode executor.
//!
//! Safe Mode reapplies the active collection through the normal durable apply
//! pipeline. The collection remains the user's requested state; the pipeline
//! filters its managed members to `is_safe` only while Safe Mode is enabled.

use tauri::Manager;

use crate::modules::settings::application::config::ConfigService;
use crate::shared::errors::AppError;

fn require<'a, T: Send + Sync + 'static>(
    app: &'a tauri::AppHandle,
    what: &str,
) -> Result<tauri::State<'a, T>, AppError> {
    app.try_state::<T>()
        .ok_or_else(|| AppError::Internal(format!("{what} not available")))
}

pub(super) async fn execute_toggle_safe_mode(app: &tauri::AppHandle) -> Result<String, AppError> {
    let config_state = require::<ConfigService>(app, "ConfigService")?;
    let pool_state = require::<sqlx::SqlitePool>(app, "SqlitePool")?;
    let watcher_state = require::<
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >(app, "WatcherState")?;
    let mutation_coordinator = require::<crate::modules::mutation::coordinator::MutationCoordinator>(
        app,
        "MutationCoordinator",
    )?;

    let current_settings = config_state.get_settings();
    let game = current_settings
        .active_game()
        .ok_or_else(|| AppError::Internal("No active game selected".to_string()))?;
    let game_id = game.id.clone();
    let mods_path = game.mod_path.clone();
    let target_safe_mode = !current_settings.safety.runtime_safe_mode_for(&game_id);
    let runtime =
        crate::modules::collections::adapters::sqlite::runtime::get(pool_state.inner(), &game_id)
            .await?;
    let mut active_collection_id = runtime
        .as_ref()
        .and_then(|state| state.active_collection_id.clone())
        .or_else(|| {
            runtime
                .as_ref()
                .and_then(|state| state.draft_collection_id.clone())
        });
    if active_collection_id.is_none() {
        active_collection_id =
            crate::modules::collections::application::collection::capture_last_changes_if_needed(
                pool_state.inner(),
                &game_id,
            )
            .await?;
    }
    let Some(active_collection_id) = active_collection_id else {
        // An empty managed library is still a valid runtime intent: the next
        // import or preset apply must inherit this filter instead of forcing
        // the user to press F5 again. There is no folder mutation to journal.
        config_state.set_runtime_safe_mode(&game_id, target_safe_mode)?;
        let sync = match crate::modules::system::application::app::post_apply::request_overlay_sync_for_game(
            pool_state.inner(),
            &config_state,
            &game_id,
            crate::modules::system::application::app::post_apply::OverlaySyncCause::SafeModeChanged,
        )
        .await
        .and_then(
            crate::modules::system::application::app::post_apply::RuntimeSyncResult::ensure_success,
        ) {
            Ok(result) => format!("overlay synchronized ({:?})", result.reload),
            Err(error) => format!("sync pending: {error}"),
        };
        return Ok(format!(
            "Safe Mode {} (no managed mods, {sync})",
            if target_safe_mode { "on" } else { "off" },
        ));
    };

    let preflight_paths =
        crate::modules::collections::application::collection::collection_preflight_scope_paths(
            pool_state.inner(),
            &game_id,
            &active_collection_id,
            &mods_path,
        )
        .await?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        app,
        pool_state.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let disk_reconcile = require::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >(app, "DiskReconcileState")?;
    let mutation_lease = disk_reconcile
        .acquire_mutation_lease(&game_id, mutation_coordinator.inner_lock())
        .await?;

    // The apply pipeline needs the target eligibility immediately, but the
    // committed setting must not lead the durable filesystem mutation. Keep a
    // private target snapshot until that mutation has finalized successfully.
    let mut apply_settings = current_settings.clone();
    apply_settings
        .safety
        .set_runtime_safe_mode(game_id.clone(), target_safe_mode);
    let apply_result =
        crate::modules::collections::application::collection::apply_collection_durable_for_safe_mode(
            crate::modules::collections::application::collection::ApplyCollectionRequest {
                pool: pool_state.inner(),
                game_id: &game_id,
                collection_id: &active_collection_id,
                capture_last_changes: false,
                mods_path,
                suppressor: watcher_state.suppressor.clone(),
                ignore_missing: true,
                settings: apply_settings.clone(),
            },
            mutation_coordinator.inner(),
        )
        .await
        .map_err(|error| {
            AppError::Internal(format!(
                "Safe Mode mutation did not commit; requested state remains unchanged: {error}"
            ))
        })?;

    // This is deliberately after the journal-backed apply. If persistence of
    // the presentation state fails, the folder mutation is already committed;
    // report that truth instead of claiming a rollback that never happened.
    config_state
        .set_runtime_safe_mode(&game_id, target_safe_mode)
        .map_err(|error| {
            AppError::Internal(format!(
                "Safe Mode mutation committed but runtime state sync is pending: {error}"
            ))
        })?;

    drop(mutation_lease);
    crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
        app,
        pool_state.inner(),
        &game_id,
    )
    .await?;

    Ok(format!(
        "Safe Mode {} (enabled: {}, disabled: {}; overlay sync was requested by the committed collection apply)",
        if target_safe_mode { "on" } else { "off" },
        apply_result.mods_enabled,
        apply_result.mods_disabled,
    ))
}
