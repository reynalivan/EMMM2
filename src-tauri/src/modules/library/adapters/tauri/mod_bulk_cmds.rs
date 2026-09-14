use crate::modules::library::application::mods::bulk;
use crate::modules::library::application::mods::info_json;
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::reconciliation::application::disk_reconcile::emit::require_applied_reconcile;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::ValidatedPath;
use crate::shared::errors::AppError;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, State};

async fn record_bulk_toggle_result(app: &AppHandle, enabled: bool, result: &bulk::BulkResult) {
    if !enabled {
        return;
    }
    let telemetry = app
        .state::<crate::modules::system::application::telemetry::TelemetryStore>()
        .inner()
        .clone();
    for _ in &result.success {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::BulkAction,
            crate::modules::system::application::telemetry::TelemetryOutcome::Success,
            crate::modules::system::application::telemetry::TelemetryErrorCode::None,
        );
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    for _ in &result.failures {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::BulkAction,
            crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
            crate::modules::system::application::telemetry::TelemetryErrorCode::Unknown,
        );
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
    if !result.success.is_empty() && !result.failures.is_empty() {
        let event = crate::modules::system::application::telemetry::TelemetryEvent::new(
            crate::modules::system::application::telemetry::TelemetryOperation::BulkAction,
            crate::modules::system::application::telemetry::TelemetryOutcome::Partial,
            crate::modules::system::application::telemetry::TelemetryErrorCode::Unknown,
        );
        let _ = telemetry
            .record_rollup(env!("CARGO_PKG_VERSION"), event, chrono::Utc::now())
            .await;
    }
}

/// Cooperative cancel for the two bulk actions that walk the filesystem one
/// folder at a time. A single flag is enough: `OperationLock` already
/// serializes bulk runs, so two batches are never in flight together.
#[derive(Default)]
pub struct BulkCancelState(AtomicBool);

impl BulkCancelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears a cancel left over from an earlier batch and hands back the flag.
    /// Callers already hold the operation lock, so this cannot wipe a cancel
    /// aimed at a run that is still going.
    fn begin(&self) -> &AtomicBool {
        self.0.store(false, Ordering::SeqCst);
        &self.0
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn apply_committed_reconcile(
    result: &mut bulk::BulkResult,
    settlement: crate::modules::reconciliation::application::disk_reconcile::emit::CommittedReconcileSettlement,
) {
    if let Some(reconcile) = settlement.reconcile {
        result
            .collection_impact
            .merge(reconcile.collection_reference_impact);
    }
    result.sync_warning = settlement.sync_warning;
}

fn partition_toggle_paths(
    validated: Vec<ValidatedPath>,
    conflicts: &[crate::modules::reconciliation::application::disk_reconcile::types::FolderNameConflictGroup],
) -> (Vec<ValidatedPath>, Vec<bulk::BulkActionError>) {
    let (blocked, safe): (Vec<_>, Vec<_>) = validated.into_iter().partition(|path| {
        let canonical_path = path.to_string_lossy().into_owned();
        crate::modules::reconciliation::application::disk_reconcile::emit::conflicts_intersect_paths(
            conflicts,
            std::slice::from_ref(&canonical_path),
        )
    });
    let failures = blocked
        .into_iter()
        .map(|path| bulk::BulkActionError {
            path: path.original().to_string(),
            error: crate::modules::reconciliation::application::disk_reconcile::emit::folder_conflict_mutation_error(),
        })
        .collect();
    (safe, failures)
}

/// A UI selection can contain the same folder through multiple paths, or a
/// parent and its child. Rename planning is only well-defined for disjoint
/// folders, so canonical aliases collapse while overlapping entries are both
/// returned as per-item failures.
fn normalize_toggle_paths(
    validated: Vec<ValidatedPath>,
) -> (Vec<ValidatedPath>, Vec<bulk::BulkActionError>) {
    let mut seen = HashSet::new();
    let unique = validated
        .into_iter()
        .filter(|path| seen.insert(physical_path_key(path.as_ref())))
        .collect::<Vec<_>>();
    let unique_by_path = unique
        .iter()
        .enumerate()
        .map(|(index, path)| (physical_path_key(path.as_ref()), index))
        .collect::<HashMap<_, _>>();
    let mut overlapping = vec![false; unique.len()];
    for (index, path) in unique.iter().enumerate() {
        let mut ancestor = path.as_ref().parent();
        while let Some(parent) = ancestor {
            if let Some(other_index) = unique_by_path.get(&physical_path_key(parent)) {
                overlapping[index] = true;
                overlapping[*other_index] = true;
            }
            ancestor = parent.parent();
        }
    }
    let failures = overlapping
        .iter()
        .enumerate()
        .filter_map(|(index, is_overlapping)| {
            is_overlapping.then_some(bulk::BulkActionError {
                path: unique[index].original().to_string(),
                error: AppError::Validation(
                    "Bulk toggle cannot include both a folder and one of its descendants"
                        .to_string(),
                ),
            })
        })
        .collect::<Vec<_>>();
    let safe = unique
        .into_iter()
        .enumerate()
        .filter_map(|(index, path)| (!overlapping[index]).then_some(path))
        .collect();
    (safe, failures)
}

/// A canonical OS path is physical identity: unlike a logical mod identity it
/// must keep the `DISABLED` prefix, otherwise two conflict candidates could be
/// silently collapsed into one requested mutation.
fn physical_path_key(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let regular = if let Some(unc) = normalized.strip_prefix("//?/UNC/") {
        format!("//{unc}")
    } else if let Some(path) = normalized.strip_prefix("//?/") {
        path.to_string()
    } else {
        normalized
    };
    regular
        .chars()
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn append_preflight_failures(result: &mut bulk::BulkResult, failures: Vec<bulk::BulkActionError>) {
    result.failures.extend(failures);
}

/// Stop the running bulk toggle/delete after the item in flight. Work already
/// done stays done — the trailing reconcile still converges the DB.
#[specta::specta]
#[tauri::command]
pub async fn bulk_cancel(cancel_state: State<'_, BulkCancelState>) -> Result<(), AppError> {
    cancel_state.cancel();
    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_toggle_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, AppError> {
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    // Containment failures reject the entire request. Stale folders are local
    // failures, so a multi-select remains useful when one entry disappeared.
    let (validated, stale_paths) =
        crate::platform::fs::guard::validate_mod_toggle_paths(&config, &game_id, &paths)?;
    let (validated, overlapping_paths) = normalize_toggle_paths(validated);
    let mut preflight_failures = stale_paths
        .into_iter()
        .map(|(path, error)| bulk::BulkActionError { path, error })
        .collect::<Vec<_>>();
    preflight_failures.extend(overlapping_paths);
    if validated.is_empty() {
        let result =
            bulk::BulkResult::new(Vec::new(), preflight_failures).with_execution_state(false, 0, 0);
        record_bulk_toggle_result(&app, diagnostics_enabled, &result).await;
        return Ok(result);
    }
    let preflight_paths = validated
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let (validated, folder_conflict_failures) =
        partition_toggle_paths(validated, &preflight.folder_conflicts);
    preflight_failures.extend(folder_conflict_failures);
    if validated.is_empty() {
        let result =
            bulk::BulkResult::new(Vec::new(), preflight_failures).with_execution_state(false, 0, 0);
        record_bulk_toggle_result(&app, diagnostics_enabled, &result).await;
        return Ok(result);
    }

    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let validated_paths = validated
        .iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect::<Vec<_>>();
    let prepared = bulk::prepare_bulk_toggle(&validated_paths, enable);
    let planned_steps = prepared.planned_steps();
    if planned_steps.is_empty() {
        let _lock = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata,
            )
            .await?;
        let cancel = cancel_state.begin();
        let mut result = bulk::execute_prepared_bulk_toggle(&app, &state, &prepared, cancel).result;
        append_preflight_failures(&mut result, preflight_failures);
        record_bulk_toggle_result(&app, diagnostics_enabled, &result).await;
        return Ok(result);
    }

    let planned_sequences = prepared.planned_sequences();
    let journal_steps = planned_steps
        .into_iter()
        .map(|(sequence, old_path, new_path)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, old_path, new_path)
        })
        .collect();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "bulk-toggle",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let cancel = cancel_state.begin();
    let mut execution = bulk::execute_prepared_bulk_toggle(&app, &state, &prepared, cancel);
    append_preflight_failures(&mut execution.result, preflight_failures);

    for sequence in &planned_sequences {
        if execution.applied_sequences.contains(sequence) {
            mutation_lease.mark_step_applied(*sequence)?;
        } else {
            mutation_lease.mark_step_rolled_back(*sequence)?;
        }
    }

    if execution.applied_sequences.is_empty() {
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        record_bulk_toggle_result(&app, diagnostics_enabled, &execution.result).await;
        return Ok(execution.result);
    }

    let reconcile_result = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        &app,
        pool.inner(),
        &game_id,
        &mutation_lease,
    )
    .await;

    match reconcile_result.and_then(require_applied_reconcile) {
        Ok(reconcile_result) => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(Ok(reconcile_result));
            apply_committed_reconcile(&mut execution.result, settlement);
            record_bulk_toggle_result(&app, diagnostics_enabled, &execution.result).await;
            Ok(execution.result)
        }
        Err(error) => {
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) =
                bulk::rollback_prepared_bulk_toggle(&state, &prepared, &execution.applied_sequences)
            {
                let combined = format!("{error}; bulk toggle rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for sequence in &execution.applied_sequences {
                mutation_lease.mark_step_rolled_back(*sequence)?;
            }
            if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &game_id,
                &mutation_lease,
            )
            .await.and_then(require_applied_reconcile)
            {
                let combined =
                    format!("{error}; rollback projection failed: {rollback_reconcile_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            mutation_lease.finish_rollback()?;
            Err(error)
        }
    }
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_delete_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
) -> Result<bulk::BulkResult, AppError> {
    // Required, like `delete_mod`: it names the mods root the paths must sit
    // inside, and the game whose index rows may be pruned. Optional, it let a
    // caller skip containment entirely.
    let (validated, stale_paths) =
        crate::platform::fs::guard::validate_mod_toggle_paths(&config, &game_id, &paths)?;
    let (validated, overlapping_paths) = normalize_toggle_paths(validated);
    let mut preflight_failures = stale_paths
        .into_iter()
        .map(|(path, error)| bulk::BulkActionError { path, error })
        .collect::<Vec<_>>();
    preflight_failures.extend(overlapping_paths);
    if validated.is_empty() {
        return Ok(
            bulk::BulkResult::new(Vec::new(), preflight_failures).with_execution_state(false, 0, 0)
        );
    }
    let preflight_paths = validated
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;

    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let prepared = bulk::prepare_bulk_delete(&validated)?;
    let planned_sequences = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, _, _)| sequence)
        .collect::<Vec<_>>();
    let journal_steps = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, source, quarantine)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, source, quarantine)
        })
        .collect();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "bulk-delete",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let cancel = cancel_state.begin();
    let mut execution = bulk::execute_prepared_bulk_delete(&app, &state, &prepared, cancel);
    append_preflight_failures(&mut execution.result, preflight_failures);
    for sequence in &planned_sequences {
        if execution.applied_sequences.contains(sequence) {
            mutation_lease.mark_step_applied(*sequence)?;
        } else {
            mutation_lease.mark_step_rolled_back(*sequence)?;
        }
    }
    if execution.applied_sequences.is_empty() {
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Ok(execution.result);
    }
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        &app,
        pool.inner(),
        &game_id,
        &mutation_lease,
    )
    .await
    .and_then(require_applied_reconcile);
    let reconcile = match reconcile {
        Ok(reconcile) => reconcile,
        Err(error) => {
            mutation_lease.begin_rollback()?;
            if let Err(rollback_error) =
                bulk::rollback_prepared_bulk_delete(&state, &prepared, &execution.applied_sequences)
            {
                let combined = format!("{error}; bulk delete rollback failed: {rollback_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            for sequence in &execution.applied_sequences {
                mutation_lease.mark_step_rolled_back(*sequence)?;
            }
            if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                &app,
                pool.inner(),
                &game_id,
                &mutation_lease,
            )
            .await.and_then(require_applied_reconcile) {
                let combined =
                    format!("{error}; rollback projection failed: {rollback_reconcile_error}");
                mutation_lease.fail(combined.clone())?;
                return Err(AppError::Io(combined));
            }
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
    };
    mutation_lease.mark_db_committed()?;
    mutation_lease.commit()?;
    execution
        .result
        .collection_impact
        .merge(reconcile.collection_reference_impact);
    if let Err(error) = bulk::finalize_prepared_bulk_delete(&prepared, &execution.applied_sequences)
    {
        execution.result.sync_warning = Some(
            crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning {
                kind: crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::CleanupPending,
                message: error.to_string(),
            },
        );
    }
    Ok(execution.result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_update_info(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, AppError> {
    if update.is_safe.is_some() {
        return Err(AppError::Validation(
            "Safety changes must use bulk_set_mod_safety".to_string(),
        ));
    }
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result = bulk::bulk_update_info(&validated, update).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_set_mod_safety(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    safe: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let resolved = bulk::resolve_safety_targets(pool.inner(), &game_id, &validated).await?;
    let suppression = watcher.suppressor.suppress_paths(
        resolved
            .targets
            .iter()
            .map(|target| std::path::Path::new(&target.disk_path)),
    );
    let mut result = bulk::bulk_set_safety(pool.inner(), &game_id, resolved, safe).await?;
    drop(suppression);
    drop(lock);

    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_toggle_favorite(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result =
        bulk::bulk_toggle_favorite(&pool, game_id.clone(), folder_paths, favorite).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_pin_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata)
        .await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result = bulk::bulk_pin(&pool, game_id.clone(), folder_paths, pin).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn folder_conflicts_become_per_item_failures_without_dropping_safe_paths() {
        use crate::modules::reconciliation::application::disk_reconcile::types::{
            FolderNameConflictCandidate, FolderNameConflictGroup,
        };
        use crate::modules::settings::application::config::GameConfig;

        let pool = crate::test_utils::init_test_db().await.pool;
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let mods_root = temp_dir.path().join("Mods");
        let conflicted = mods_root.join("DISABLED Conflict");
        let safe = mods_root.join("DISABLED Safe");
        std::fs::create_dir_all(&conflicted).expect("conflicted folder");
        std::fs::create_dir_all(&safe).expect("safe folder");

        let config = ConfigService::new_for_test_async(pool).await;
        let mut settings = config.get_settings();
        settings.games.push(GameConfig {
            id: "game-1".to_string(),
            name: "Test Game".to_string(),
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            instance_path: mods_root.clone(),
            mod_path: mods_root.clone(),
            ready_to_move_path: None,
            launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
            game_exe: Some(mods_root.join("game.exe")),
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        });
        config.save_settings(settings).expect("save settings");

        let requested = vec!["DISABLED Conflict".to_string(), "DISABLED Safe".to_string()];
        let validated = crate::platform::fs::guard::validate_paths(&config, "game-1", &requested)
            .expect("paths should validate");
        let conflicts = vec![FolderNameConflictGroup {
            group_id: "conflict".to_string(),
            identity: "conflict".to_string(),
            display_name: "Conflict".to_string(),
            candidates: vec![FolderNameConflictCandidate {
                path: conflicted
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                folder_name: "DISABLED Conflict".to_string(),
                base_name: "Conflict".to_string(),
                is_enabled: false,
            }],
        }];

        let (safe_paths, failures) = partition_toggle_paths(validated, &conflicts);

        assert_eq!(safe_paths.len(), 1);
        assert_eq!(safe_paths[0].original(), "DISABLED Safe");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].path, "DISABLED Conflict");
    }

    #[test]
    fn bulk_cancel_state_is_observed_and_reset_for_the_next_batch() {
        let state = BulkCancelState::new();

        assert!(!state.begin().load(Ordering::SeqCst));
        state.cancel();
        assert!(state.0.load(Ordering::SeqCst));
        assert!(!state.begin().load(Ordering::SeqCst));
    }

    #[test]
    fn preflight_failures_do_not_change_execution_progress_totals() {
        let mut result = bulk::BulkResult::new(vec!["done".to_string()], Vec::new())
            .with_execution_state(true, 1, 3);
        append_preflight_failures(
            &mut result,
            vec![bulk::BulkActionError {
                path: "missing".to_string(),
                error: AppError::Io("Mod folder is no longer available".to_string()),
            }],
        );

        assert_eq!(result.processed_count, 1);
        assert_eq!(result.unprocessed_count, 2);
        assert_eq!(result.failures.len(), 1);
    }

    #[test]
    fn physical_selection_keys_keep_enabled_and_disabled_conflict_candidates_distinct() {
        assert_ne!(
            physical_path_key(std::path::Path::new("C:/Mods/DISABLED Blue")),
            physical_path_key(std::path::Path::new("C:/Mods/Blue")),
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn bulk_toggle_deduplicates_aliases_and_rejects_nested_targets() {
        use crate::modules::settings::application::config::GameConfig;

        let pool = crate::test_utils::init_test_db().await.pool;
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let mods_root = temp_dir.path().join("Mods");
        let parent = mods_root.join("Parent");
        let child = parent.join("Child");
        let sibling = mods_root.join("Sibling");
        std::fs::create_dir_all(&child).expect("nested folders");
        std::fs::create_dir_all(&sibling).expect("sibling folder");

        let config = ConfigService::new_for_test_async(pool).await;
        let mut settings = config.get_settings();
        settings.games.push(GameConfig {
            id: "game-1".to_string(),
            name: "Test Game".to_string(),
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            instance_path: mods_root.clone(),
            mod_path: mods_root.clone(),
            ready_to_move_path: None,
            launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
            game_exe: Some(mods_root.join("game.exe")),
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        });
        config.save_settings(settings).expect("save settings");

        let aliases = vec![
            "Sibling".to_string(),
            sibling
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ];
        let (validated, failures) =
            crate::platform::fs::guard::validate_mod_toggle_paths(&config, "game-1", &aliases)
                .expect("aliases stay inside the root");
        let (safe, overlaps) = normalize_toggle_paths(validated);
        assert!(failures.is_empty());
        assert!(overlaps.is_empty());
        assert_eq!(safe.len(), 1, "canonical aliases execute only once");

        let nested = vec!["Parent".to_string(), "Parent/Child".to_string()];
        let (validated, failures) =
            crate::platform::fs::guard::validate_mod_toggle_paths(&config, "game-1", &nested)
                .expect("nested paths are individually contained");
        let (safe, overlaps) = normalize_toggle_paths(validated);
        assert!(failures.is_empty());
        assert!(safe.is_empty());
        assert_eq!(
            overlaps
                .iter()
                .map(|failure| failure.path.as_str())
                .collect::<Vec<_>>(),
            vec!["Parent", "Parent/Child"]
        );
    }
}
