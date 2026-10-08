use tauri::Manager;

use crate::modules::collections::application::collection;
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
    let config = require::<ConfigService>(app, "ConfigService")?;
    let pool = require::<sqlx::SqlitePool>(app, "SqlitePool")?;
    let watcher = require::<crate::modules::workspace::application::scanner::watcher::WatcherState>(
        app,
        "WatcherState",
    )?;
    let coordinator = require::<crate::modules::mutation::coordinator::MutationCoordinator>(
        app,
        "MutationCoordinator",
    )?;
    let initial = config.get_settings();
    let game = initial
        .active_game()
        .ok_or_else(|| AppError::Validation("No active game selected".to_string()))?;
    let game_id = game.id.clone();
    let mods_path = game.mod_path.clone();
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &game_id,
        crate::modules::mutation::api::ImmutableMutationKind::SafeMode,
    )?;
    let root_proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&mods_path)?;
    let snapshot_lease = crate::modules::collections::api::acquire_current_snapshot_lease(
        app,
        pool.inner(),
        coordinator.inner(),
        &game_id,
    )
    .await?;
    root_proof.validate(&mods_path)?;
    crate::modules::reconciliation::api::ensure_projection_epoch(
        config.inner(),
        &game_id,
        root_proof.identity(),
    )?;
    let settings = config.get_settings();
    let previous_enabled = settings.safety.runtime_safe_mode_for(&game_id);
    let task_id =
        collection::prepare_safe_mode_transition(pool.inner(), &game_id, previous_enabled).await?;
    drop(snapshot_lease);

    let outcome = async {
        let preflight_paths = collection::safe_mode_preflight_scope_paths(pool.inner(), &task_id, &mods_path)
            .await?.ok_or_else(|| AppError::Validation("Safe Mode intent is missing".to_string()))?;
        crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
            app, pool.inner(), &game_id, Some(&preflight_paths),
        ).await?;
        let disk_reconcile = require::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>(app, "DiskReconcileState")?;
        let mutation_lease = disk_reconcile.acquire_ready_mutation_lease(&game_id, coordinator.inner_lock()).await?;
        root_proof.validate(&mods_path)?;
        crate::modules::reconciliation::api::ensure_projection_epoch(config.inner(), &game_id, root_proof.identity())?;
        let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(pool.inner(), &task_id)
            .await?.ok_or_else(|| AppError::Validation("Safe Mode task is missing".to_string()))?;
        let result = collection::execute_safe_mode_transition(
            collection::ApplyCollectionRequest {
                pool: pool.inner(), game_id: &game_id, collection_id: "", capture_last_changes: false,
                mods_path: mods_path.clone(), suppressor: watcher.suppressor.clone(),
                ignore_missing: true, settings,
            },
            &task_id, task.final_active_collection_id, config.inner(), coordinator.inner(),
        ).await?;
        let generation = crate::modules::reconciliation::api::enqueue_runtime_sync_for_rewrites(
            app, pool.inner(), &game_id, &mods_path,
            crate::modules::reconciliation::api::RuntimeSyncCause::SafeModeChanged,
            &result.runtime_path_rewrites,
        );
        drop(mutation_lease);
        Ok(format!("Safe Mode {} (enabled: {}, disabled: {}; overlay queued as generation {generation})",
            if previous_enabled { "off" } else { "on" }, result.mods_enabled, result.mods_disabled))
    }.await;
    if outcome.is_err() {
        collection::release_safe_mode_task_if_running(pool.inner(), &task_id).await?;
    }
    outcome
}
