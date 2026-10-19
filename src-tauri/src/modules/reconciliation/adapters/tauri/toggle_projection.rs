use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

use crate::modules::mutation::api::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileResult, DiskReconcileStatus,
};
use crate::modules::settings::api::config::ConfigService;
use crate::modules::workspace::api::scanner::watcher::WatcherState;
use crate::shared::errors::AppError;

static RUNNING_SWITCH_PROJECTIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn is_toggle_projection_operation(
    operation: &crate::modules::mutation::journal::Operation,
    game_id: &str,
) -> bool {
    operation.game_id == game_id
        && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
}

#[cfg(test)]
#[path = "toggle_projection/tests.rs"]
mod tests;

use crate::modules::reconciliation::application::toggle_projection::{
    legacy_projection_matches_root, projection_rename_proof, rename_confirmation_operation_ids,
    settle_repaired_projection_operations, trusted_projection_paths,
    unprojected_toggle_operation_ids, ProjectionRenameProof,
};

#[cfg(test)]
use crate::modules::reconciliation::application::toggle_projection::proven_projection_events;

enum ToggleProjectionAttempt {
    Reconciled(Box<DiskReconcileResult>, Vec<String>),
    Replaced(Vec<String>),
}

#[derive(Clone, serde::Serialize)]
struct WorkspaceSwitchProjected {
    game_id: String,
    source_epoch: String,
    disk_revision: u64,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct WorkspaceSwitchSnapshot {
    game_id: String,
    source_epoch: String,
    disk_revision: u64,
    projected_revision: u64,
    projection_repair_reason: Option<String>,
}

pub(crate) fn projection_source_epoch(
    config: &ConfigService,
    game_id: &str,
) -> Result<String, AppError> {
    let root = config
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
        &root,
    )
    .ok_or_else(|| AppError::Io(format!("Mods root is unavailable: {}", root.display())))
}

pub(crate) fn ensure_projection_epoch(
    config: &ConfigService,
    game_id: &str,
    expected: &str,
) -> Result<(), AppError> {
    if projection_source_epoch(config, game_id)? != expected {
        return Err(AppError::Io(
            "Mods root identity changed during the operation; refresh this game before retrying"
                .to_string(),
        ));
    }
    Ok(())
}

use crate::modules::reconciliation::adapters::sqlite::{
    persist_projection_checkpoint, read_projection_checkpoint,
};

async fn checkpoint_current_projection(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    expected_epoch: &str,
    revision: Option<u64>,
) -> Result<(), AppError> {
    ensure_projection_epoch(config, game_id, expected_epoch)?;
    if let Some(revision) = revision {
        persist_projection_checkpoint(pool, game_id, expected_epoch, revision).await?;
        ensure_projection_epoch(config, game_id, expected_epoch)?;
    }
    Ok(())
}

pub(crate) async fn complete_reconciled_toggle_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    coordinator: &MutationCoordinator,
    game_id: &str,
    expected_epoch: &str,
    pending_ids: &[String],
    revision: Option<u64>,
) -> Result<(), AppError> {
    let pending = coordinator.pending_disk_commits()?;
    let current = pending
        .iter()
        .filter(|operation| pending_ids.contains(&operation.id))
        .cloned()
        .collect::<Vec<_>>();
    let root = app
        .state::<ConfigService>()
        .mods_root_for(game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    if current.iter().any(|operation| {
        operation
            .source_epoch
            .as_deref()
            .is_some_and(|epoch| epoch != expected_epoch)
    }) || (current
        .iter()
        .any(|operation| operation.source_epoch.is_none())
        && !legacy_projection_matches_root(&current, &root))
    {
        return Err(AppError::Io(
            "Pending disk commits belong to an unverified root epoch; recovery is required"
                .to_string(),
        ));
    }
    let current = settle_repaired_projection_operations(
        pool,
        coordinator,
        game_id,
        expected_epoch,
        &root,
        &current,
    )
    .await?;
    if let Some((hole, reason)) =
        coordinator.earliest_toggle_projection_repair(game_id, expected_epoch)?
    {
        return Err(AppError::Io(format!(
            "Disk projection requires repair at revision {hole}: {reason}"
        )));
    }
    let revision = revision
        .into_iter()
        .chain(
            current
                .iter()
                .filter_map(|operation| operation.disk_revision),
        )
        .max();
    let completed_revision = coordinator
        .toggle_projection_lineage_in_epoch(game_id, expected_epoch)?
        .into_iter()
        .filter(|operation| {
            operation.status == crate::modules::mutation::journal::OperationStatus::Completed
        })
        .filter_map(|operation| operation.disk_revision)
        .max()
        .unwrap_or(0);
    let revision = Some(revision.unwrap_or(0).max(completed_revision));
    checkpoint_current_projection(
        &app.state::<ConfigService>(),
        pool,
        game_id,
        expected_epoch,
        revision,
    )
    .await?;
    coordinator.complete_disk_projection(pending_ids)?;
    publish_completed_toggle_projection(app, pool, game_id, expected_epoch, revision);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_workspace_switch_snapshot(
    game_id: String,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    coordinator: State<'_, MutationCoordinator>,
) -> Result<WorkspaceSwitchSnapshot, AppError> {
    let source_epoch = projection_source_epoch(config.inner(), &game_id)?;
    let mut projected_revision =
        read_projection_checkpoint(pool.inner(), &game_id, &source_epoch).await?;
    let repair = coordinator.earliest_toggle_projection_repair(&game_id, &source_epoch)?;
    if let Some((revision, _)) = &repair {
        projected_revision = projected_revision.min(revision.saturating_sub(1));
    }
    let disk_revision =
        coordinator.latest_toggle_disk_revision_in_epoch(&game_id, Some(&source_epoch))?;
    ensure_projection_epoch(config.inner(), &game_id, &source_epoch)?;
    Ok(WorkspaceSwitchSnapshot {
        game_id,
        source_epoch,
        disk_revision,
        projected_revision,
        projection_repair_reason: repair.map(|(_, reason)| reason),
    })
}

fn running_switch_projections() -> &'static Mutex<HashSet<String>> {
    RUNNING_SWITCH_PROJECTIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn projection_revision_before_repairs(
    coordinator: &MutationCoordinator,
    game_id: &str,
    epoch: &str,
    revision: Option<u64>,
) -> Result<Option<u64>, AppError> {
    Ok(
        match coordinator.earliest_toggle_projection_repair(game_id, epoch)? {
            Some((hole, _)) => revision.map(|revision| revision.min(hole.saturating_sub(1))),
            None => revision,
        },
    )
}

pub(crate) fn publish_completed_toggle_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
    disk_revision: Option<u64>,
) {
    if disk_revision.is_none() {
        return;
    }
    if let Err(error) =
        ensure_projection_epoch(&app.state::<ConfigService>(), game_id, source_epoch)
    {
        log::warn!("Discarding stale-root projection publication for '{game_id}': {error}");
        return;
    }
    crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
        app,
        pool,
        game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
        crate::modules::reconciliation::api::RuntimeSyncRequest::Full,
    );
    if let Some(disk_revision) = disk_revision {
        if let Err(error) = app.emit(
            "workspace_switch:projected",
            WorkspaceSwitchProjected {
                game_id: game_id.to_string(),
                source_epoch: source_epoch.to_string(),
                disk_revision,
            },
        ) {
            log::warn!("Could not emit workspace projection revision for '{game_id}': {error}");
        }
    }
}

pub(crate) fn queue_toggle_projection(
    app: tauri::AppHandle,
    pool: sqlx::SqlitePool,
    game_id: String,
) {
    app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>()
        .projection_wakeup(&game_id).notify_one();
    let mut running = crate::shared::sync::lock(running_switch_projections());
    if !running.insert(game_id.clone()) {
        return;
    }
    drop(running);
    tauri::async_runtime::spawn(async move {
        run_workspace_switch_projection(&app, &pool, &game_id).await;
    });
}

async fn project_toggle_scope<S, SF, F, FF>(
    paths: Option<Vec<String>>,
    scoped: S,
    full: F,
) -> Result<DiskReconcileResult, AppError>
where
    S: FnOnce(Vec<String>) -> SF,
    SF: std::future::Future<Output = Result<DiskReconcileResult, AppError>>,
    F: FnOnce() -> FF,
    FF: std::future::Future<Output = Result<DiskReconcileResult, AppError>>,
{
    match paths {
        Some(paths) => scoped(paths).await,
        None => full().await,
    }
}

async fn run_workspace_switch_projection(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
) {
    let mut retry_delay = Duration::from_millis(75);
    let mut failures = 0u32;
    let state = app.state::<crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState>();
    let wakeup = state.projection_wakeup(game_id);
    loop {
        tokio::select! {
            _ = tokio::time::sleep(retry_delay) => {},
            _ = wakeup.notified() => {},
        }
        let notified = wakeup.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let coordinator = app.state::<MutationCoordinator>();
        let state = app.state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>();
        let lease_started = Instant::now();
        let game_guard = state.game_lock(game_id).lock_owned().await;
        let Some(operation_guard) = coordinator.inner_lock().try_acquire_for_reconcile() else {
            drop(game_guard);
            retry_delay = Duration::from_millis(75);
            continue;
        };
        let lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_reconcile_guard(game_guard, operation_guard);
        let lease_wait_ms = lease_started.elapsed().as_millis();
        let mut pending = match coordinator.pending_disk_commits() {
            Ok(pending) => pending
                .into_iter()
                .filter(|operation| is_toggle_projection_operation(operation, game_id))
                .collect::<Vec<_>>(),
            Err(error) => {
                log::error!(
                    "Could not inspect pending workspace projection for '{game_id}': {error}"
                );
                drop(lease);
                notified.await;
                retry_delay = Duration::from_millis(75);
                continue;
            }
        };
        if pending.is_empty() {
            drop(lease);
            let mut running = crate::shared::sync::lock(running_switch_projections());
            match coordinator.pending_disk_commits() {
                Ok(remaining)
                    if !remaining
                        .iter()
                        .any(|operation| is_toggle_projection_operation(operation, game_id)) =>
                {
                    running.remove(game_id);
                    return;
                }
                Ok(_) => continue,
                Err(error) => {
                    log::error!(
                        "Could not settle workspace projection worker for '{game_id}': {error}"
                    );
                    retry_delay = Duration::from_secs(1);
                    continue;
                }
            }
        }
        let projection_epoch = match projection_source_epoch(&app.state::<ConfigService>(), game_id)
        {
            Ok(epoch) => epoch,
            Err(error) => {
                log::warn!("Workspace projection source is unavailable for '{game_id}': {error}");
                drop(lease);
                notified.await;
                retry_delay = Duration::from_millis(75);
                continue;
            }
        };
        let Some(root) = app.state::<ConfigService>().mods_root_for(game_id) else {
            drop(lease);
            notified.await;
            continue;
        };
        let candidates = pending
            .iter()
            .filter(|operation| {
                operation
                    .source_epoch
                    .as_deref()
                    .is_none_or(|epoch| epoch == projection_epoch)
            })
            .cloned()
            .collect::<Vec<_>>();
        let legacy_valid = legacy_projection_matches_root(&candidates, &root);
        let mut isolation_failed = false;
        for operation in &pending {
            let belongs = operation
                .source_epoch
                .as_deref()
                .map_or(legacy_valid, |epoch| epoch == projection_epoch);
            if !belongs {
                if let Err(error) = coordinator.isolate_projection_for_repair(&operation.id, "Root epoch mismatch or unproven legacy identity; disk commit preserved for manual repair") {
                    log::error!("Could not isolate stale-root projection: {error}");
                    isolation_failed = true;
                }
            }
        }
        pending.retain(|operation| {
            operation
                .source_epoch
                .as_deref()
                .map_or(legacy_valid, |epoch| epoch == projection_epoch)
        });
        if pending.is_empty() {
            drop(lease);
            if isolation_failed {
                notified.await;
            }
            continue;
        }
        let pending_ids = pending
            .iter()
            .map(|operation| operation.id.clone())
            .collect::<Vec<_>>();
        let projected_revision = pending
            .iter()
            .filter_map(|operation| operation.disk_revision)
            .max();
        let authority_marker = app
            .try_state::<ConfigService>()
            .and_then(|config| config.mods_root_for(game_id))
            .and_then(|mods_root| {
                app.try_state::<WatcherState>()
                    .and_then(|watcher| watcher.current_session_for_root(&mods_root))
                    .and_then(|session| {
                        state
                            .authority_event_generation(game_id, session.generation())
                            .map(|generation| (mods_root, session.generation(), generation))
                    })
            });
        if authority_marker.is_none() {
            if let Err(error) = coordinator.note_disk_projection_failure(
                &pending_ids,
                "Projection parked: watcher coverage is unavailable",
            ) {
                log::error!("Could not record parked projection: {error}");
            }
            drop(lease);
            notified.await;
            retry_delay = Duration::from_millis(75);
            continue;
        }
        let watcher = app.state::<WatcherState>();
        let echo_watermark = watcher.suppressor.expected_echo_watermark();
        let trusted_authority =
            authority_marker
                .as_ref()
                .and_then(|(mods_root, watcher_session, _)| {
                    state.trusted_internal_mutation_evidence(game_id, mods_root, *watcher_session)
                });
        let changed_paths = pending
            .iter()
            .flat_map(|operation| &operation.steps)
            .flat_map(|step| [&step.old_path, &step.new_path])
            .flatten()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let scoped_paths = trusted_projection_paths(&pending);
        let scope_mode = if scoped_paths.is_some() {
            "scoped"
        } else {
            "full_unproven_scope"
        };
        let projection_started = Instant::now();
        log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} operations={} mode={scope_mode} lease_wait_ms={lease_wait_ms}", pending_ids.len());
        let projection = tokio::select! {
            biased;
            _ = coordinator.inner_lock().wait_for_foreground_intent() => None,
            result = async {
                let proof_pending = pending.clone();
                let proof_root = root.clone();
                let proof = tokio::task::spawn_blocking(move || projection_rename_proof(&proof_pending, &proof_root)).await??;
                let rename_events = match proof {
                    ProjectionRenameProof::Proven(events) => events,
                    ProjectionRenameProof::Replaced(ids) => return Ok(ToggleProjectionAttempt::Replaced(ids)),
                };
                let result = project_toggle_scope(
                    scoped_paths,
                    |paths| crate::modules::reconciliation::application::disk_reconcile::emit::run_trusted_internal_disk_reconcile_under_lease(app, pool, game_id, paths, rename_events, &lease),
                    || crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_full_internal_disk_reconcile_under_lease(app, pool, game_id, &lease),
                ).await?;
                let invalid = if result.status.applied() {
                    unprojected_toggle_operation_ids(pool, game_id, &pending, &root).await?
                } else {
                    Vec::new()
                };
                Ok::<_, AppError>(ToggleProjectionAttempt::Reconciled(Box::new(result), invalid))
            } => Some(result),
        };
        let Some(projection) = projection else {
            log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} outcome=yielded elapsed_ms={}", projection_started.elapsed().as_millis());
            drop(lease);
            retry_delay = Duration::from_millis(75);
            continue;
        };
        match projection {
            Ok(ToggleProjectionAttempt::Replaced(ids)) => {
                let message = "Owned projection target was replaced; verified disk commit preserved for repair";
                for id in ids {
                    if let Err(error) = coordinator.isolate_projection_for_repair(&id, message) {
                        log::error!(
                            "Could not retain replaced projection repair evidence: {error}"
                        );
                    }
                }
                drop(lease);
                retry_delay = Duration::from_millis(75);
            }
            Ok(ToggleProjectionAttempt::Reconciled(mut result, invalid)) => {
                log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} outcome={:?} mode={scope_mode} elapsed_ms={}", result.status, projection_started.elapsed().as_millis());
                if result.status == DiskReconcileStatus::SourceUnavailable {
                    let message = result.error_message.as_deref().unwrap_or(
                        "Projection source is unavailable; verified disk changes remain committed",
                    );
                    if let Err(error) =
                        coordinator.note_disk_projection_failure(&pending_ids, message)
                    {
                        log::error!("Could not record unavailable projection source: {error}");
                    }
                    drop(lease);
                    retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                    continue;
                }
                if result.status == DiskReconcileStatus::NeedsRenameConfirmation {
                    let message = "Folder changes are committed on disk; resolve ambiguous rename ownership before synchronization can finish";
                    let affected = rename_confirmation_operation_ids(
                        &pending,
                        &root,
                        &result.rename_confirmations,
                    );
                    for id in &affected {
                        if let Err(error) = coordinator.isolate_projection_for_repair(id, message) {
                            log::error!(
                                "Could not retain rename-confirmation repair evidence: {error}"
                            );
                        }
                    }
                    drop(lease);
                    retry_delay = if affected.is_empty() {
                        next_projection_retry_delay(&mut failures, retry_delay)
                    } else {
                        Duration::from_millis(75)
                    };
                    continue;
                }
                if !invalid.is_empty() {
                    let message = "Owned disk rename is not indexed; resolve the affected folder conflict before capturing a collection";
                    log::warn!("Preserving disk commits with incomplete owned projection for '{game_id}': {message}");
                    for id in invalid {
                        if let Err(journal_error) =
                            coordinator.isolate_projection_for_repair(&id, message)
                        {
                            log::error!(
                                "Could not isolate incomplete owned projection: {journal_error}"
                            );
                        }
                    }
                    drop(lease);
                    retry_delay = Duration::from_millis(75);
                    continue;
                }
                let authority_accepted = if let Some(authority) = &trusted_authority {
                    state.mark_trusted_internal_mutation_reconciled(
                        authority,
                        &result,
                        &changed_paths,
                    )
                } else if let Some((mods_root, watcher_session, observed_generation)) =
                    &authority_marker
                {
                    state.mark_authority_reconciled(
                        game_id,
                        mods_root,
                        *watcher_session,
                        *observed_generation,
                        &result,
                        &changed_paths,
                    )
                } else {
                    false
                };
                if !authority_accepted {
                    log::debug!(
                        "Workspace disk projection for '{game_id}' awaits current watcher authority"
                    );
                    drop(lease);
                    retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                    continue;
                }
                result.warnings.retain(|warning| {
                    warning.kind
                        != crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileWarningKind::AuthorityPending
                });
                let acknowledged_revision = match projection_revision_before_repairs(
                    &coordinator,
                    game_id,
                    &projection_epoch,
                    projected_revision,
                ) {
                    Ok(revision) => revision,
                    Err(error) => {
                        log::error!(
                            "Could not inspect workspace projection repair boundary: {error}"
                        );
                        drop(lease);
                        retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                        continue;
                    }
                };
                let checkpoint = checkpoint_current_projection(
                    &app.state::<ConfigService>(),
                    pool,
                    game_id,
                    &projection_epoch,
                    acknowledged_revision,
                )
                .await;
                #[cfg(debug_assertions)]
                let settlement_started = Instant::now();
                let settlement =
                    checkpoint.and_then(|()| coordinator.complete_disk_projection(&pending_ids));
                #[cfg(debug_assertions)]
                log::info!("toggle projection settlement game_id={} operations={} success={} journal_snapshot_ms={}",
                    game_id, pending_ids.len(), settlement.is_ok(), settlement_started.elapsed().as_millis());
                if let Err(error) = settlement {
                    log::error!(
                        "Could not complete workspace disk projection for '{game_id}': {error}"
                    );
                    drop(lease);
                    retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
                    continue;
                }
                if let Some(session) = watcher.current_session_for_root(&root) {
                    watcher
                        .suppressor
                        .mark_rename_echoes_reconciled_through(&session, echo_watermark);
                }
                match ensure_projection_epoch(
                    &app.state::<ConfigService>(),
                    game_id,
                    &projection_epoch,
                ) {
                    Ok(()) => {
                        if let Err(error) = app.emit("disk_reconcile:result", &result) {
                            log::warn!("Could not emit completed workspace projection for '{game_id}': {error}");
                        }
                    }
                    Err(error) => log::warn!(
                        "Discarding stale-root reconcile publication for '{game_id}': {error}"
                    ),
                }
                drop(lease);
                publish_completed_toggle_projection(
                    app,
                    pool,
                    game_id,
                    &projection_epoch,
                    acknowledged_revision,
                );
                retry_delay = Duration::from_millis(75);
                failures = 0;
            }
            Err(error) => {
                log::debug!("WorkspaceProjection game_id={game_id} epoch={projection_epoch} disk_revision={projected_revision:?} outcome=retry mode={scope_mode} elapsed_ms={}", projection_started.elapsed().as_millis());
                log::error!("Workspace disk projection remains pending for '{game_id}': {error}");
                if let Err(journal_error) =
                    coordinator.note_disk_projection_failure(&pending_ids, &error.to_string())
                {
                    log::error!("Could not record pending projection failure for '{game_id}': {journal_error}");
                }
                drop(lease);
                retry_delay = next_projection_retry_delay(&mut failures, retry_delay);
            }
        }
    }
}

fn next_projection_retry_delay(failures: &mut u32, previous: Duration) -> Duration {
    *failures = failures.saturating_add(1);
    if *failures >= 6 {
        Duration::from_secs(30)
    } else {
        (previous * 2).min(Duration::from_secs(5))
    }
}
