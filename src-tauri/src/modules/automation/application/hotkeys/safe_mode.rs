use tauri::Manager;

use super::manager::HotkeyGameContext;
use super::notice::{HotkeyActionOutcome, HotkeyNoticeKind};
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

pub(super) async fn execute_toggle_safe_mode(
    app: &tauri::AppHandle,
    context: &HotkeyGameContext,
) -> Result<HotkeyActionOutcome, AppError> {
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
    context.validate(&initial)?;
    let game_id = context.game_id().to_string();
    let mods_path = context.mods_path().to_path_buf();
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &game_id,
        crate::modules::mutation::api::ImmutableMutationKind::SafeMode,
    )?;
    let snapshot_lease = crate::modules::collections::api::acquire_current_snapshot_lease(
        app,
        pool.inner(),
        coordinator.inner(),
        &game_id,
    )
    .await?;
    context.validate(&config.get_settings())?;
    crate::modules::reconciliation::api::ensure_projection_epoch(
        config.inner(),
        &game_id,
        context.root_identity(),
    )?;
    let settings = config.get_settings();
    context.validate(&settings)?;
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
        let settings = config.get_settings();
        context.validate(&settings)?;
        crate::modules::reconciliation::api::ensure_projection_epoch(config.inner(), &game_id, context.root_identity())?;
        let task = crate::modules::workspace::adapters::sqlite::task::get_task_by_id(pool.inner(), &task_id)
            .await?.ok_or_else(|| AppError::Validation("Safe Mode task is missing".to_string()))?;
        let authoritative_collection_id =
            crate::modules::collections::adapters::sqlite::runtime::get(pool.inner(), &game_id)
                .await?
                .and_then(|runtime| runtime.active_collection_id);
        if authoritative_collection_id.as_deref() != task.final_active_collection_id.as_deref() {
            return Err(AppError::Validation(
                "Collection state changed while Safe Mode was waiting; retry the hotkey".to_string(),
            ));
        }
        let collections = collection::list_collections(pool.inner(), &game_id).await?;
        let collection_name = authoritative_collection_id.as_deref().and_then(|id| {
            collections
                .iter()
                .find(|collection| collection.id == id)
                .map(|collection| collection.name.clone())
        });
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
        let safe_mode = !previous_enabled;
        Ok(HotkeyActionOutcome {
            kind: HotkeyNoticeKind::SafeModeChanged,
            previous_collection: collection_name.clone(),
            collection: collection_name,
            safe_mode,
            message: None,
            runtime_generation: Some(generation),
            log_summary: format!("Safe Mode {} (enabled: {}, disabled: {}; overlay queued as generation {generation})",
                if safe_mode { "on" } else { "off" }, result.mods_enabled, result.mods_disabled),
        })
    }.await;
    match outcome {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            if let Err(release_error) =
                collection::release_safe_mode_task_if_running(pool.inner(), &task_id).await
            {
                log::error!(
                    "Safe Mode hotkey failed ({error}); cleanup for task '{task_id}' also failed: {release_error}"
                );
            }
            Err(error)
        }
    }
}
