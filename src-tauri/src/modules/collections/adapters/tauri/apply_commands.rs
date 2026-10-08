use super::{
    collection_apply_changed_disk, ensure_current_runtime_snapshot_preflight,
    record_collection_operation,
};
use crate::modules::collections::application::collection;
use crate::modules::collections::domain::collection::ApplyResult;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use tauri::Emitter;
use tauri::{AppHandle, State};
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary: states plus the IPC payload.
pub async fn apply_collection(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher_state: State<
        '_,
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    collection_id: String,
    ignore_missing: Option<bool>,
) -> Result<ApplyResult, AppError> {
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &game_id,
        crate::modules::mutation::api::ImmutableMutationKind::CollectionApply,
    )?;
    let started_at = std::time::Instant::now();
    let settings = config.get_settings();
    let diagnostics_enabled = settings.diagnostics.telemetry_enabled;
    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| {
            AppError::RuntimeState(crate::shared::errors::RuntimeStateError::GameNotFound {
                game_id: game_id.clone(),
            })
        })?;
    let mods_path = game.mod_path.clone();
    let root_proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&mods_path)?;
    let mut mutation_lease = None;
    for _ in 0..3 {
        let source_epoch = root_proof.identity();
        root_proof.validate(&mods_path)?;
        crate::modules::reconciliation::api::ensure_projection_epoch(
            config.inner(),
            &game_id,
            source_epoch,
        )?;
        let pending_before = op_lock.pending_toggle_disk_commit_ids(&game_id)?;
        if !pending_before.is_empty() {
            ensure_current_runtime_snapshot_preflight(&app, pool.inner(), &game_id).await?;
        }
        let preflight_paths = collection::collection_preflight_scope_paths(
            pool.inner(),
            &game_id,
            &collection_id,
            &mods_path,
        )
        .await?;
        crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
            &app,
            pool.inner(),
            &game_id,
            Some(&preflight_paths),
        )
        .await?;
        let lease = disk_reconcile
            .acquire_nested_mutation_lease(&game_id, op_lock.inner())
            .await?;
        root_proof.validate(&mods_path)?;
        crate::modules::reconciliation::api::ensure_projection_epoch(
            config.inner(),
            &game_id,
            source_epoch,
        )?;
        let pending_after = op_lock.pending_toggle_disk_commit_ids(&game_id)?;
        if pending_after.iter().all(|id| pending_before.contains(id)) {
            let projected_revision = op_lock
                .pending_disk_commits()?
                .into_iter()
                .filter(|operation| pending_after.contains(&operation.id))
                .filter_map(|operation| operation.disk_revision)
                .max();
            crate::modules::reconciliation::api::complete_reconciled_toggle_projection(
                &app,
                pool.inner(),
                op_lock.inner(),
                &game_id,
                source_epoch,
                &pending_after,
                projected_revision,
            )
            .await?;
            mutation_lease = Some(lease);
            break;
        }
        drop(lease);
    }
    let mutation_lease = mutation_lease.ok_or_else(|| {
        AppError::Io(
            "Mods changed repeatedly while applying the collection; retry once switching settles"
                .to_string(),
        )
    })?;

    let mut result = collection::apply_collection_durable(
        collection::ApplyCollectionRequest {
            pool: pool.inner(),
            game_id: &game_id,
            collection_id: &collection_id,
            capture_last_changes: true,
            mods_path: mods_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: ignore_missing.unwrap_or(false),
            settings,
        },
        op_lock.inner(),
    )
    .await;

    if let Ok(applied) = &mut result {
        if collection_apply_changed_disk(applied) {
            let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
                crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(
                    &app,
                    pool.inner(),
                    &game_id,
                    &mutation_lease,
                )
                .await,
            );
            if let Some(reconcile) = settlement.reconcile {
                if let Err(error) = app.emit("disk_reconcile:result", &reconcile) {
                    log::warn!("Could not emit collection disk reconcile result: {error}");
                }
            }
            applied.sync_warning = settlement.sync_warning;
        }
        crate::modules::reconciliation::api::enqueue_runtime_sync_for_rewrites(
            &app,
            pool.inner(),
            &game_id,
            &mods_path,
            crate::modules::reconciliation::api::RuntimeSyncCause::CollectionApplied,
            &applied.runtime_path_rewrites,
        );
    }

    drop(mutation_lease);
    record_collection_operation(
        &app,
        diagnostics_enabled,
        crate::modules::system::application::telemetry::TelemetryOperation::CollectionApply,
        result.is_ok(),
        started_at,
    )
    .await;
    result.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn restore_last_changes(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    config: State<'_, crate::modules::settings::application::config::ConfigService>,
    watcher_state: State<
        '_,
        crate::modules::workspace::application::scanner::watcher::WatcherState,
    >,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
) -> Result<ApplyResult, AppError> {
    let _admission = crate::modules::mutation::api::admit_immutable_mutation(
        &game_id,
        crate::modules::mutation::api::ImmutableMutationKind::CollectionApply,
    )?;
    let started_at = std::time::Instant::now();
    let runtime =
        crate::modules::collections::adapters::sqlite::runtime::get(pool.inner(), &game_id)
            .await?
            .ok_or_else(|| AppError::Validation("No Last changes snapshot exists".to_string()))?;
    let draft_id = runtime
        .draft_collection_id
        .ok_or_else(|| AppError::Validation("No Last changes snapshot exists".to_string()))?;
    let settings = config.get_settings();
    let diagnostics_enabled = settings.diagnostics.telemetry_enabled;
    let game = settings
        .games
        .iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}' not found")))?;
    let mods_path = game.mod_path.clone();
    let root_proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&mods_path)?;
    let preflight_paths =
        collection::collection_preflight_scope_paths(pool.inner(), &game_id, &draft_id, &mods_path)
            .await?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let mutation_lease = disk_reconcile
        .acquire_nested_mutation_lease(&game_id, op_lock.inner())
        .await?;
    root_proof.validate(&mods_path)?;
    crate::modules::reconciliation::api::ensure_projection_epoch(
        config.inner(),
        &game_id,
        root_proof.identity(),
    )?;
    let restored_baseline = collection::valid_active_baseline(
        pool.inner(),
        &game_id,
        runtime.draft_base_collection_id.as_deref(),
    )
    .await?;
    let result = collection::restore_collection_with_baseline_durable(
        collection::ApplyCollectionRequest {
            pool: pool.inner(),
            game_id: &game_id,
            collection_id: &draft_id,
            capture_last_changes: false,
            mods_path: mods_path.clone(),
            suppressor: watcher_state.suppressor.clone(),
            ignore_missing: false,
            settings: settings.clone(),
        },
        restored_baseline,
        op_lock.inner(),
    )
    .await?;
    crate::modules::reconciliation::api::enqueue_runtime_sync_for_rewrites(
        &app,
        pool.inner(),
        &game_id,
        &mods_path,
        crate::modules::reconciliation::api::RuntimeSyncCause::CollectionApplied,
        &result.runtime_path_rewrites,
    );
    drop(mutation_lease);
    record_collection_operation(
        &app,
        diagnostics_enabled,
        crate::modules::system::application::telemetry::TelemetryOperation::Restore,
        true,
        started_at,
    )
    .await;
    Ok(result)
}
