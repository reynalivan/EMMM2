use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::modules::collections::application::collection;
use crate::modules::collections::domain::collection::ApplyResult;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::shared::errors::AppError;

pub(super) async fn record_collection_operation(
    app: &AppHandle,
    enabled: bool,
    operation: crate::modules::system::application::telemetry::TelemetryOperation,
    succeeded: bool,
    started_at: std::time::Instant,
) {
    if !enabled || !succeeded {
        return;
    }
    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .inner()
        .clone();
    let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
        operation,
        crate::modules::system::application::telemetry::TelemetryOutcome::Success,
        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
    )
    .with_duration(started_at.elapsed());
    let _ = telemetry
        .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
        .await;
}

pub(super) fn collection_apply_changed_disk(result: &ApplyResult) -> bool {
    result.mods_enabled + result.mods_disabled > 0 || !result.runtime_path_rewrites.is_empty()
}

pub(super) async fn ensure_current_runtime_snapshot_preflight(
    app: &AppHandle,
    pool: &SqlitePool,
    game_id: &str,
) -> Result<(), AppError> {
    let result = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_for_paths(
        app, pool, game_id, None,
    )
    .await?;
    if result.status
        == crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::AppliedWithFolderConflicts
    {
        let active_paths = collection::active_runtime_snapshot_scope_paths(pool, game_id).await?;
        if current_runtime_snapshot_conflicts_block(&result.folder_conflicts, &active_paths) {
            return Err(
                crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error(),
            );
        }
    }
    Ok(())
}

pub(super) async fn acquire_current_snapshot_guard(
    app: &AppHandle,
    pool: &SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
) -> Result<CurrentSnapshotGuard, AppError> {
    let admission = crate::modules::mutation::api::admit_immutable_mutation(
        game_id,
        crate::modules::mutation::api::ImmutableMutationKind::CollectionCapture,
    )?;
    let mutation = acquire_current_snapshot_lease(app, pool, coordinator, game_id).await?;
    Ok(CurrentSnapshotGuard {
        _mutation: mutation,
        _admission: admission,
    })
}

pub(crate) async fn acquire_current_snapshot_lease(
    app: &AppHandle,
    pool: &SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    AppError,
> {
    let disk_reconcile = app.state::<
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >();
    for _ in 0..3 {
        let source_epoch = crate::modules::reconciliation::api::projection_source_epoch(
            &app.state::<crate::modules::settings::application::config::ConfigService>(),
            game_id,
        )?;
        let pending_before = coordinator.pending_toggle_disk_commit_ids(game_id)?;
        ensure_current_runtime_snapshot_preflight(app, pool, game_id).await?;
        let guard = disk_reconcile
            .acquire_nested_mutation_lease(game_id, coordinator)
            .await?;
        let pending_after = coordinator.pending_toggle_disk_commit_ids(game_id)?;
        if pending_after.iter().all(|id| pending_before.contains(id)) {
            let projected_revision = coordinator
                .pending_disk_commits()?
                .into_iter()
                .filter(|operation| pending_after.contains(&operation.id))
                .filter_map(|operation| operation.disk_revision)
                .max();
            crate::modules::reconciliation::api::complete_reconciled_toggle_projection(
                app,
                pool,
                coordinator,
                game_id,
                &source_epoch,
                &pending_after,
                projected_revision,
            )
            .await?;
            return Ok(guard);
        }
        drop(guard);
    }
    Err(AppError::Io(
        "Mods changed repeatedly while capturing the collection; retry once switching settles"
            .to_string(),
    ))
}

pub(super) struct CurrentSnapshotGuard {
    _mutation: crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    _admission: crate::modules::mutation::api::ImmutableMutationPermit,
}

fn current_runtime_snapshot_conflicts_block(
    conflicts: &[crate::modules::reconciliation::application::disk_reconcile::types::FolderNameConflictGroup],
    active_paths: &[String],
) -> bool {
    crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
        conflicts,
        active_paths,
    )
}

// ============================================================================
// Runtime state commands
// ============================================================================

#[path = "runtime_commands.rs"]
mod runtime_commands;
pub use runtime_commands::*;

#[path = "capture_commands.rs"]
mod capture_commands;
pub use capture_commands::*;

#[path = "apply_commands.rs"]
mod apply_commands;
pub use apply_commands::*;

#[path = "preview_commands.rs"]
mod preview_commands;
pub use preview_commands::*;

#[path = "recovery_commands.rs"]
mod recovery_commands;
pub use recovery_commands::*;

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;

#[path = "save_changes.rs"]
mod save_changes;
pub use save_changes::*;
