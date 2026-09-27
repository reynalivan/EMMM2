use crate::modules::library::application::mods::bulk;
use crate::modules::library::application::mods::info_json;
use crate::modules::mutation::api::StepSettlement;
use crate::modules::mutation::coordinator::{IntentTarget, MutationCoordinator};
use crate::modules::reconciliation::application::disk_reconcile::emit::require_applied_reconcile;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::ValidatedPath;
use crate::shared::errors::AppError;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

const MAX_BULK_PATHS: usize = 10_000;
const BULK_TOGGLE_CHUNK_SIZE: usize = 32;

fn validate_snapshot_identities(
    expected_identities: Option<&[(String, String)]>,
) -> Result<(), AppError> {
    if let Some(expected_identities) = expected_identities {
        crate::modules::workspace::application::explorer::listing::validate_workspace_explorer_selection_identities(
            expected_identities,
        )?;
    }
    Ok(())
}

async fn acquire_snapshot_game_guard(
    disk_reconcile: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    game_id: &str,
    expected_identities: Option<&[(String, String)]>,
) -> Result<tokio::sync::OwnedMutexGuard<()>, AppError> {
    let game_guard = disk_reconcile.game_lock(game_id).lock_owned().await;
    validate_snapshot_identities(expected_identities)?;
    Ok(game_guard)
}

fn validate_bulk_size(paths: &[String]) -> Result<(), AppError> {
    if paths.len() > MAX_BULK_PATHS {
        return Err(AppError::Validation(format!(
            "Bulk operations support at most {MAX_BULK_PATHS} paths"
        )));
    }
    Ok(())
}

fn record_bulk_toggle_result(app: &AppHandle, enabled: bool, result: &bulk::BulkResult) {
    if !enabled {
        return;
    }
    let Some(telemetry) =
        app.try_state::<crate::modules::system::application::telemetry::TelemetrySink>()
    else {
        return;
    };
    let success = crate::modules::system::application::telemetry::TelemetryEvent::new(
        crate::modules::system::application::telemetry::TelemetryOperation::BulkAction,
        crate::modules::system::application::telemetry::TelemetryOutcome::Success,
        crate::modules::system::application::telemetry::TelemetryErrorCode::None,
    );
    let failed = crate::modules::system::application::telemetry::TelemetryEvent::new(
        crate::modules::system::application::telemetry::TelemetryOperation::BulkAction,
        crate::modules::system::application::telemetry::TelemetryOutcome::Failed,
        crate::modules::system::application::telemetry::TelemetryErrorCode::Unknown,
    );
    let mut events = std::iter::repeat_n(success, result.success.len())
        .chain(std::iter::repeat_n(failed, result.failures.len()))
        .collect::<Vec<_>>();
    if !result.success.is_empty() && !result.failures.is_empty() {
        events.push(
            crate::modules::system::application::telemetry::TelemetryEvent::new(
                crate::modules::system::application::telemetry::TelemetryOperation::BulkAction,
                crate::modules::system::application::telemetry::TelemetryOutcome::Partial,
                crate::modules::system::application::telemetry::TelemetryErrorCode::Unknown,
            ),
        );
    }
    telemetry.try_enqueue(events);
}

#[derive(Default)]
pub struct BulkCancelState {
    operations: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

struct BulkCancellation<'a> {
    state: &'a BulkCancelState,
    operation_id: String,
    flag: Arc<AtomicBool>,
}

impl BulkCancellation<'_> {
    fn flag(&self) -> &AtomicBool {
        &self.flag
    }
}

impl Drop for BulkCancellation<'_> {
    fn drop(&mut self) {
        let mut operations = crate::shared::sync::lock(&self.state.operations);
        if operations
            .get(&self.operation_id)
            .is_some_and(|current| Arc::ptr_eq(current, &self.flag))
        {
            operations.remove(&self.operation_id);
        }
    }
}

impl BulkCancelState {
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&self, operation_id: &str) -> Result<BulkCancellation<'_>, AppError> {
        if operation_id.is_empty()
            || operation_id.len() > 128
            || !operation_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(AppError::Validation(
                "Bulk operation ID must contain 1-128 ASCII letters, digits, '-' or '_'"
                    .to_string(),
            ));
        }
        let flag = Arc::new(AtomicBool::new(false));
        let mut operations = crate::shared::sync::lock(&self.operations);
        if operations.contains_key(operation_id) {
            return Err(AppError::Validation(format!(
                "Bulk operation '{operation_id}' is already registered"
            )));
        }
        operations.insert(operation_id.to_string(), Arc::clone(&flag));
        Ok(BulkCancellation {
            state: self,
            operation_id: operation_id.to_string(),
            flag,
        })
    }

    fn cancel(&self, operation_id: &str) {
        if let Some(flag) = crate::shared::sync::lock(&self.operations).get(operation_id) {
            flag.store(true, Ordering::SeqCst);
        }
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

fn toggle_step_paths(steps: &[bulk::PreparedToggleStep]) -> Vec<String> {
    steps
        .iter()
        .flat_map(|step| {
            [
                step.old_path.to_string_lossy().into_owned(),
                step.new_path.to_string_lossy().into_owned(),
            ]
        })
        .collect()
}

fn trusted_bulk_toggle_scope(steps: &[bulk::PreparedToggleStep], mods_root: &Path) -> bool {
    !steps.is_empty()
        && steps.iter().all(|step| {
            step.old_path.starts_with(mods_root)
                && step.new_path.starts_with(mods_root)
                && step.old_path.parent() == step.new_path.parent()
                && crate::shared::path_key::folder_path_key(
                    &step
                        .old_path
                        .strip_prefix(mods_root)
                        .unwrap_or(&step.old_path)
                        .to_string_lossy(),
                    None,
                ) == crate::shared::path_key::folder_path_key(
                    &step
                        .new_path
                        .strip_prefix(mods_root)
                        .unwrap_or(&step.new_path)
                        .to_string_lossy(),
                    None,
                )
        })
}

fn append_preflight_failures(result: &mut bulk::BulkResult, failures: Vec<bulk::BulkActionError>) {
    result.failures.extend(failures);
}

fn settle_superseded_bulk_items(
    mut result: bulk::BulkResult,
    requested_count: usize,
    superseded_count: usize,
) -> bulk::BulkResult {
    result.cancelled |= superseded_count > 0;
    result.processed_count = result.success.len() + result.failures.len();
    result.unprocessed_count = requested_count.saturating_sub(result.processed_count);
    result
}

fn emit_toggle_progress(
    app: &AppHandle,
    operation_id: &str,
    enable: bool,
    current: usize,
    total: usize,
    active: bool,
    cancelled: bool,
) {
    let label = if active {
        if enable {
            "common:bulk_progress.enabling"
        } else {
            "common:bulk_progress.disabling"
        }
    } else if cancelled {
        "common:bulk_progress.cancelled"
    } else {
        "common:bulk_progress.done"
    };
    let _ = app.emit(
        "bulk-progress",
        bulk::BulkProgressPayload {
            operation_id: operation_id.to_string(),
            cancellable: active,
            label: label.to_string(),
            current,
            total,
            active,
        },
    );
}

fn merge_toggle_chunk_result(aggregate: &mut bulk::BulkResult, result: bulk::BulkResult) {
    aggregate.cancelled |= result.cancelled;
    aggregate.success.extend(result.success);
    aggregate.failures.extend(result.failures);
    aggregate.path_rewrites.extend(result.path_rewrites);
    aggregate.collection_impact.merge(result.collection_impact);
    if let Some(revision) = result.disk_revision {
        aggregate.disk_revision = Some(aggregate.disk_revision.unwrap_or(0).max(revision));
    }
    if result.sync_warning.is_some() {
        aggregate.sync_warning = result.sync_warning;
    }
}

fn current_toggle_paths(
    admission: &crate::modules::mutation::coordinator::IntentAdmission,
    game_id: &str,
    paths: &[String],
) -> Vec<String> {
    paths
        .iter()
        .filter(|path| admission.is_current_path(game_id, Path::new(path)))
        .cloned()
        .collect()
}

/// Stop the running bulk toggle/delete after the item in flight. Work already
/// done stays done — the trailing reconcile still converges the DB.
#[specta::specta]
#[tauri::command]
pub async fn bulk_cancel(
    cancel_state: State<'_, BulkCancelState>,
    operation_id: String,
) -> Result<(), AppError> {
    cancel_state.cancel(&operation_id);
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
    game_id: String,
    paths: Vec<String>,
    enable: bool,
    operation_id: String,
    intent_revision: Option<u64>,
) -> Result<bulk::BulkResult, AppError> {
    let op_lock = app.state::<MutationCoordinator>();
    bulk_toggle_mods_impl(
        app.clone(),
        config,
        pool,
        state,
        disk_reconcile,
        op_lock,
        game_id,
        paths,
        enable,
        operation_id,
        None,
        intent_revision,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bulk_toggle_mods_from_snapshot(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
    operation_id: String,
    expected_identities: Vec<(String, String)>,
    intent_revision: Option<u64>,
) -> Result<bulk::BulkResult, AppError> {
    bulk_toggle_mods_impl(
        app,
        config,
        pool,
        state,
        disk_reconcile,
        op_lock,
        game_id,
        paths,
        enable,
        operation_id,
        Some(expected_identities),
        intent_revision,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn bulk_toggle_mods_impl(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
    operation_id: String,
    expected_identities: Option<Vec<(String, String)>>,
    intent_revision: Option<u64>,
) -> Result<bulk::BulkResult, AppError> {
    validate_bulk_size(&paths)?;
    let admission = op_lock.admit_intents(
        &game_id,
        intent_revision,
        paths.iter().cloned().map(IntentTarget::ModPath),
    );
    let cancel_state = app.state::<BulkCancelState>();
    let cancellation = cancel_state.register(&operation_id)?;
    let _foreground_intent = op_lock.inner_lock().foreground_intent();

    let (validated, stale_paths) =
        crate::platform::fs::guard::validate_mod_toggle_paths(&config, &game_id, &paths)?;
    let (validated, overlapping_paths) = normalize_toggle_paths(validated);
    let mut failures = stale_paths
        .into_iter()
        .map(|(path, error)| bulk::BulkActionError { path, error })
        .chain(overlapping_paths)
        .filter(|failure| admission.is_current_path(&game_id, Path::new(&failure.path)))
        .collect::<Vec<_>>();
    let valid_paths = validated
        .into_iter()
        .map(|path| path.original().to_string())
        .collect::<Vec<_>>();
    let mut aggregate = bulk::BulkResult::new(Vec::new(), std::mem::take(&mut failures));
    emit_toggle_progress(&app, &operation_id, enable, 0, paths.len(), true, false);

    'chunks: for chunk in valid_paths.chunks(BULK_TOGGLE_CHUNK_SIZE) {
        if cancellation.flag().load(Ordering::Relaxed) {
            aggregate.cancelled = true;
            break;
        }
        let mut chunk_paths = chunk.to_vec();
        loop {
            let chunk_keys = chunk_paths.iter().collect::<HashSet<_>>();
            let chunk_identities = expected_identities.as_ref().map(|identities| {
                identities
                    .iter()
                    .filter(|(path, _)| chunk_keys.contains(path))
                    .cloned()
                    .collect::<Vec<_>>()
            });
            let chunk_result = bulk_toggle_mods_chunk(
                app.clone(),
                config.clone(),
                pool.clone(),
                state.clone(),
                disk_reconcile.clone(),
                op_lock.clone(),
                game_id.clone(),
                chunk_paths.clone(),
                enable,
                &operation_id,
                chunk_identities,
                &admission,
                cancellation.flag(),
            )
            .await;
            match chunk_result {
                Ok(result)
                    if result.cancelled
                        && result.processed_count == 0
                        && result.failures.is_empty()
                        && !cancellation.flag().load(Ordering::Relaxed) =>
                {
                    let current = current_toggle_paths(&admission, &game_id, &chunk_paths);
                    if current.len() == chunk_paths.len() {
                        aggregate.cancelled = true;
                        break 'chunks;
                    }
                    chunk_paths = current;
                    if chunk_paths.is_empty() {
                        break;
                    }
                }
                Ok(result) => {
                    merge_toggle_chunk_result(&mut aggregate, result);
                    break;
                }
                Err(error) => {
                    aggregate
                        .failures
                        .extend(chunk_paths.into_iter().map(|path| bulk::BulkActionError {
                            path,
                            error: error.clone(),
                        }));
                    aggregate.cancelled = true;
                    break 'chunks;
                }
            }
        }
        aggregate.processed_count = aggregate.success.len() + aggregate.failures.len();
        emit_toggle_progress(
            &app,
            &operation_id,
            enable,
            aggregate.processed_count,
            paths.len(),
            true,
            false,
        );
        tokio::task::yield_now().await;
    }

    aggregate.processed_count = aggregate.success.len() + aggregate.failures.len();
    aggregate.unprocessed_count = paths.len().saturating_sub(aggregate.processed_count);
    aggregate.cancelled |= aggregate.unprocessed_count > 0;
    emit_toggle_progress(
        &app,
        &operation_id,
        enable,
        aggregate.processed_count,
        paths.len(),
        false,
        aggregate.cancelled,
    );
    Ok(aggregate)
}

#[allow(clippy::too_many_arguments)]
async fn bulk_toggle_mods_chunk(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
    operation_id: &str,
    expected_identities: Option<Vec<(String, String)>>,
    admission: &crate::modules::mutation::coordinator::IntentAdmission,
    cancel: &AtomicBool,
) -> Result<bulk::BulkResult, AppError> {
    let started_at = Instant::now();
    let diagnostics_enabled = config.get_settings().diagnostics.telemetry_enabled;
    // Containment failures reject the entire request. Stale folders are local
    // failures, so a multi-select remains useful when one entry disappeared.
    let (validated, stale_paths) =
        crate::platform::fs::guard::validate_mod_toggle_paths(&config, &game_id, &paths)?;
    let (validated, overlapping_paths) = normalize_toggle_paths(validated);
    let (stale_paths, superseded_stale_paths): (Vec<_>, Vec<_>) = stale_paths
        .into_iter()
        .partition(|(path, _)| admission.is_current_path(&game_id, Path::new(path)));
    let (overlapping_paths, superseded_overlaps): (Vec<_>, Vec<_>) = overlapping_paths
        .into_iter()
        .partition(|failure| admission.is_current_path(&game_id, Path::new(&failure.path)));
    let mut superseded_count = superseded_stale_paths.len() + superseded_overlaps.len();
    let mut preflight_failures = stale_paths
        .into_iter()
        .map(|(path, error)| bulk::BulkActionError { path, error })
        .collect::<Vec<_>>();
    preflight_failures.extend(overlapping_paths);
    if validated.is_empty() {
        let result = settle_superseded_bulk_items(
            bulk::BulkResult::new(Vec::new(), preflight_failures),
            paths.len(),
            superseded_count,
        );
        record_bulk_toggle_result(&app, diagnostics_enabled, &result);
        return Ok(result);
    }
    let _foreground_intent = op_lock.inner_lock().foreground_intent();
    let lock_wait_started_at = Instant::now();
    let game_guard = disk_reconcile.game_lock(&game_id).lock_owned().await;
    let (validated, superseded_paths): (Vec<_>, Vec<_>) = validated
        .into_iter()
        .partition(|path| admission.is_current_path(&game_id, Path::new(path.original())));
    superseded_count += superseded_paths.len();
    let current_identities = expected_identities.as_deref().map(|identities| {
        identities
            .iter()
            .filter(|(path, _)| admission.is_current_path(&game_id, Path::new(path)))
            .cloned()
            .collect::<Vec<_>>()
    });
    validate_snapshot_identities(current_identities.as_deref())?;
    if validated.is_empty() {
        let processed = preflight_failures.len();
        let result = bulk::BulkResult::new(Vec::new(), preflight_failures).with_execution_state(
            superseded_count > 0,
            processed,
            paths.len(),
        );
        record_bulk_toggle_result(&app, diagnostics_enabled, &result);
        return Ok(result);
    }
    let lock_wait_elapsed = lock_wait_started_at.elapsed();
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_initial_recovery_allows_mutation(
        &app,
        &game_id,
    )?;
    let mods_root = config
        .mods_root_for(&game_id)
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    let initial_paths = validated
        .iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect::<Vec<_>>();
    let initial_prepared = bulk::prepare_bulk_toggle(&initial_paths, enable);
    let initial_steps = initial_prepared.planned_steps_with_identity();
    if initial_steps.is_empty() {
        let _lock = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata,
            )
            .await?;
        let mut result = bulk::execute_prepared_bulk_toggle_silent(
            &app,
            &state,
            &initial_prepared,
            cancel,
            operation_id,
        )
        .result;
        append_preflight_failures(&mut result, preflight_failures);
        let result = settle_superseded_bulk_items(result, paths.len(), superseded_count);
        record_bulk_toggle_result(&app, diagnostics_enabled, &result);
        return Ok(result);
    }
    let trusted_scope = trusted_bulk_toggle_scope(&initial_steps, &mods_root);
    let storage_fast_path = trusted_scope;
    let preflight_started_at = Instant::now();
    let validated = if storage_fast_path {
        validated
    } else {
        let preflight_guard = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::Reconciliation,
            )
            .await?;
        let preflight = crate::modules::reconciliation::application::disk_reconcile::emit::mutation_preflight_report_under_game_lock(
            &app,
            pool.inner(),
            &game_id,
            toggle_step_paths(&initial_steps),
            trusted_scope && disk_reconcile.trusted_regional_mutation_allowed(&game_id, &mods_root),
            &game_guard,
            preflight_guard.op_guard(),
        )
        .await?;
        drop(preflight_guard);
        let (validated, folder_conflict_failures) =
            partition_toggle_paths(validated, &preflight.folder_conflicts);
        preflight_failures.extend(folder_conflict_failures);
        validated
    };
    let preflight_elapsed = preflight_started_at.elapsed();
    if validated.is_empty() {
        let result = settle_superseded_bulk_items(
            bulk::BulkResult::new(Vec::new(), preflight_failures),
            paths.len(),
            superseded_count,
        );
        record_bulk_toggle_result(&app, diagnostics_enabled, &result);
        return Ok(result);
    }
    let validated_paths = validated
        .iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect::<Vec<_>>();
    let prepared = if storage_fast_path {
        initial_prepared
    } else {
        bulk::prepare_bulk_toggle(&validated_paths, enable)
    };
    let planned_steps = prepared.planned_steps_with_identity();
    if planned_steps.is_empty() {
        let _lock = op_lock
            .acquire_exempt(
                crate::modules::mutation::coordinator::MutationExemption::LibraryMetadata,
            )
            .await?;
        let mut result = bulk::execute_prepared_bulk_toggle_silent(
            &app,
            &state,
            &prepared,
            cancel,
            operation_id,
        )
        .result;
        append_preflight_failures(&mut result, preflight_failures);
        let result = settle_superseded_bulk_items(result, paths.len(), superseded_count);
        record_bulk_toggle_result(&app, diagnostics_enabled, &result);
        return Ok(result);
    }

    let planned_sequences = prepared.planned_sequences();
    let journal_steps = planned_steps
        .iter()
        .map(|step| {
            crate::modules::mutation::api::PlannedStep::rename(
                step.sequence,
                step.old_path.clone(),
                step.new_path.clone(),
            )
            .with_expected_identity(Some(step.expected_identity.clone()))
        })
        .collect();
    let journal_started_at = Instant::now();
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "bulk-toggle",
            game_id.clone(),
            journal_steps,
        ))
        .await?;
    let journal_elapsed = journal_started_at.elapsed();
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    if validated
        .iter()
        .any(|path| !admission.is_current_path(&game_id, Path::new(path.original())))
    {
        mutation_lease.begin_rollback()?;
        let settlements = planned_sequences
            .iter()
            .map(|sequence| (*sequence, StepSettlement::RolledBack))
            .collect::<Vec<_>>();
        mutation_lease.settle_steps(&settlements)?;
        mutation_lease.finish_rollback()?;
        let result = bulk::BulkResult::new(Vec::new(), Vec::new()).with_execution_state(
            true,
            0,
            paths.len(),
        );
        return Ok(result);
    }
    if let Err(error) = prepared.validate_identities() {
        mutation_lease.begin_rollback()?;
        let settlements = planned_sequences
            .iter()
            .map(|sequence| (*sequence, StepSettlement::RolledBack))
            .collect::<Vec<_>>();
        mutation_lease.settle_steps(&settlements)?;
        mutation_lease.finish_rollback()?;
        return Err(error);
    }
    let trusted_mutation = (trusted_bulk_toggle_scope(&planned_steps, &mods_root)
        && disk_reconcile.trusted_regional_mutation_allowed(&game_id, &mods_root))
    .then(|| state.current_session_for_root(&mods_root))
    .flatten()
    .and_then(|session| {
        disk_reconcile
            .trusted_internal_mutation_evidence(&game_id, &mods_root, session.generation())
            .map(|authority| (authority, session))
    });
    let expected_echo_evidence = trusted_mutation.as_ref().map(|(_, session)| {
        state.suppressor.expect_rename_echoes(
            &game_id,
            session,
            planned_steps.iter().map(|step| {
                crate::modules::workspace::application::scanner::watcher::ExpectedRenameEcho {
                    old_path: step.old_path.clone(),
                    new_path: step.new_path.clone(),
                    expected_identity: step.expected_identity.clone(),
                }
            }),
        )
    });
    let rename_started_at = Instant::now();
    let mut execution =
        bulk::execute_prepared_bulk_toggle_silent(&app, &state, &prepared, cancel, operation_id);
    let rename_elapsed = rename_started_at.elapsed();
    append_preflight_failures(&mut execution.result, preflight_failures);

    let applied_sequences = execution
        .applied_sequences
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if let Some(evidence) = &expected_echo_evidence {
        state.suppressor.retain_expected_rename_echoes(
            evidence,
            planned_steps
                .iter()
                .filter(|step| applied_sequences.contains(&step.sequence))
                .map(|step| (step.old_path.as_path(), step.new_path.as_path())),
        );
    }
    let settlements = planned_sequences
        .iter()
        .map(|sequence| {
            let settlement = if applied_sequences.contains(sequence) {
                StepSettlement::Applied
            } else {
                StepSettlement::Skipped
            };
            (*sequence, settlement)
        })
        .collect::<Vec<_>>();
    if let Err(error) = mutation_lease.settle_steps(&settlements) {
        if let Some(evidence) = &expected_echo_evidence {
            state.suppressor.discard_expected_rename_echoes(evidence);
        }
        disk_reconcile.invalidate_authority(&game_id, &mods_root);
        return Err(error);
    }

    if execution.applied_sequences.is_empty() {
        if let Some(evidence) = &expected_echo_evidence {
            state.suppressor.discard_expected_rename_echoes(evidence);
        }
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        let result = settle_superseded_bulk_items(execution.result, paths.len(), superseded_count);
        record_bulk_toggle_result(&app, diagnostics_enabled, &result);
        return Ok(result);
    }

    let disk_revision = match mutation_lease.mark_disk_committed() {
        Ok(revision) => revision,
        Err(error) => {
            let rollback_started = mutation_lease.begin_rollback();
            if let Err(rollback_error) =
                bulk::rollback_prepared_bulk_toggle(&state, &prepared, &execution.applied_sequences)
            {
                let message = format!(
                    "Bulk disk commit record failed: {error}; filesystem rollback failed: {rollback_error}"
                );
                disk_reconcile.invalidate_authority(&game_id, &mods_root);
                let _ = mutation_lease.fail(message.clone());
                return Err(AppError::Io(message));
            }
            rollback_started?;
            for sequence in &execution.applied_sequences {
                mutation_lease.mark_step_rolled_back(*sequence)?;
            }
            mutation_lease.finish_rollback()?;
            disk_reconcile.invalidate_authority(&game_id, &mods_root);
            return Err(error);
        }
    };
    execution.result.disk_revision = Some(disk_revision);
    if trusted_mutation.is_none() {
        disk_reconcile.invalidate_authority(&game_id, &mods_root);
    }
    if let Err(error) = crate::modules::system::application::app::post_apply::reserve_overlay_sync_revision_for_request(
        &game_id,
        crate::modules::reconciliation::api::RuntimeSyncRequest::Full,
    ) {
        log::error!("Could not invalidate runtime publication after bulk disk commit: {error}");
    }
    drop(mutation_lease);
    crate::modules::workspace::adapters::tauri::workspace_cmds::queue_toggle_projection(
        app.clone(),
        pool.inner().clone(),
        game_id,
    );
    log::debug!(
        "bulk toggle timing outcome=disk_committed operation_id={} rename_count={} preflight_ms={} lock_wait_ms={} journal_ms={} rename_ms={} total_ms={}",
        operation_id,
        execution.applied_sequences.len(),
        preflight_elapsed.as_millis(),
        lock_wait_elapsed.as_millis(),
        journal_elapsed.as_millis(),
        rename_elapsed.as_millis(),
        started_at.elapsed().as_millis(),
    );
    let result = settle_superseded_bulk_items(execution.result, paths.len(), superseded_count);
    record_bulk_toggle_result(&app, diagnostics_enabled, &result);
    Ok(result)
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
    operation_id: String,
) -> Result<bulk::BulkResult, AppError> {
    bulk_delete_mods_impl(
        app,
        config,
        pool,
        state,
        disk_reconcile,
        op_lock,
        cancel_state,
        game_id,
        paths,
        operation_id,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bulk_delete_mods_from_snapshot(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
    operation_id: String,
    expected_identities: Vec<(String, String)>,
) -> Result<bulk::BulkResult, AppError> {
    bulk_delete_mods_impl(
        app,
        config,
        pool,
        state,
        disk_reconcile,
        op_lock,
        cancel_state,
        game_id,
        paths,
        operation_id,
        Some(expected_identities),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn bulk_delete_mods_impl(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
    operation_id: String,
    expected_identities: Option<Vec<(String, String)>>,
) -> Result<bulk::BulkResult, AppError> {
    validate_bulk_size(&paths)?;
    let cancellation = cancel_state.register(&operation_id)?;
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

    let game_guard = acquire_snapshot_game_guard(
        disk_reconcile.inner(),
        &game_id,
        expected_identities.as_deref(),
    )
    .await?;
    let prepared = bulk::prepare_bulk_delete(&validated)?;
    let planned_sequences = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, _, _, _)| sequence)
        .collect::<Vec<_>>();
    let journal_steps = prepared
        .journal_steps()
        .into_iter()
        .map(|(sequence, source, quarantine, expected_identity)| {
            crate::modules::mutation::api::PlannedStep::rename(sequence, source, quarantine)
                .with_expected_identity(Some(expected_identity))
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
    let mut execution = bulk::execute_prepared_bulk_delete(
        &app,
        &state,
        &prepared,
        cancellation.flag(),
        &operation_id,
    );
    append_preflight_failures(&mut execution.result, preflight_failures);
    let applied_sequences = execution
        .applied_sequences
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let settlements = planned_sequences
        .iter()
        .map(|sequence| {
            let settlement = if applied_sequences.contains(sequence) {
                StepSettlement::Applied
            } else {
                StepSettlement::Skipped
            };
            (*sequence, settlement)
        })
        .collect::<Vec<_>>();
    mutation_lease.settle_steps(&settlements)?;
    if execution.applied_sequences.is_empty() {
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Ok(execution.result);
    }
    let changed_paths = prepared
        .journal_steps()
        .into_iter()
        .filter(|(sequence, _, _, _)| applied_sequences.contains(sequence))
        .flat_map(|(_, source, quarantine, _)| {
            [
                source.to_string_lossy().into_owned(),
                quarantine.to_string_lossy().into_owned(),
            ]
        })
        .collect();
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_deferred_internal_disk_reconcile_with_path_hints_under_lease(
        &app,
        pool.inner(),
        &game_id,
        changed_paths,
        Vec::new(),
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
            let rolled_back_settlements = execution
                .applied_sequences
                .iter()
                .map(|sequence| (*sequence, StepSettlement::RolledBack))
                .collect::<Vec<_>>();
            mutation_lease.settle_steps(&rolled_back_settlements)?;
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
    let generation = crate::modules::reconciliation::api::enqueue_runtime_sync_scoped(
        &app,
        pool.inner(),
        &game_id,
        crate::modules::reconciliation::api::RuntimeSyncCause::EffectiveModsChanged,
        crate::modules::reconciliation::api::runtime_sync_request_for_roots(
            &reconcile.changed_roots,
        ),
    );
    execution.result.runtime_sync_generation = Some(generation);
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
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, AppError> {
    bulk_update_info_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        paths,
        update,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bulk_update_info_from_snapshot(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
    expected_identities: Vec<(String, String)>,
) -> Result<bulk::BulkResult, AppError> {
    bulk_update_info_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        paths,
        update,
        Some(expected_identities),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn bulk_update_info_impl(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
    expected_identities: Option<Vec<(String, String)>>,
) -> Result<bulk::BulkResult, AppError> {
    validate_bulk_size(&paths)?;
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
    let game_guard = acquire_snapshot_game_guard(
        disk_reconcile.inner(),
        &game_id,
        expected_identities.as_deref(),
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
    drop(game_guard);
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
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    safe: bool,
) -> Result<bulk::BulkResult, AppError> {
    bulk_set_mod_safety_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        paths,
        safe,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bulk_set_mod_safety_from_snapshot(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    safe: bool,
    expected_identities: Vec<(String, String)>,
) -> Result<bulk::BulkResult, AppError> {
    bulk_set_mod_safety_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        paths,
        safe,
        Some(expected_identities),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn bulk_set_mod_safety_impl(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    paths: Vec<String>,
    safe: bool,
    expected_identities: Option<Vec<(String, String)>>,
) -> Result<bulk::BulkResult, AppError> {
    validate_bulk_size(&paths)?;
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    let game_guard = acquire_snapshot_game_guard(
        disk_reconcile.inner(),
        &game_id,
        expected_identities.as_deref(),
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
    drop(game_guard);

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
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, AppError> {
    bulk_toggle_favorite_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        folder_paths,
        favorite,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bulk_toggle_favorite_from_snapshot(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
    expected_identities: Vec<(String, String)>,
) -> Result<bulk::BulkResult, AppError> {
    bulk_toggle_favorite_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        folder_paths,
        favorite,
        Some(expected_identities),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn bulk_toggle_favorite_impl(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
    expected_identities: Option<Vec<(String, String)>>,
) -> Result<bulk::BulkResult, AppError> {
    validate_bulk_size(&folder_paths)?;
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let game_guard = acquire_snapshot_game_guard(
        disk_reconcile.inner(),
        &game_id,
        expected_identities.as_deref(),
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
    drop(game_guard);
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
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, AppError> {
    bulk_pin_mods_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        folder_paths,
        pin,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bulk_pin_mods_from_snapshot(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
    expected_identities: Vec<(String, String)>,
) -> Result<bulk::BulkResult, AppError> {
    bulk_pin_mods_impl(
        app,
        config,
        pool,
        watcher,
        disk_reconcile,
        op_lock,
        game_id,
        folder_paths,
        pin,
        Some(expected_identities),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn bulk_pin_mods_impl(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile: State<'_, crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>,
    op_lock: State<'_, MutationCoordinator>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
    expected_identities: Option<Vec<(String, String)>>,
) -> Result<bulk::BulkResult, AppError> {
    validate_bulk_size(&folder_paths)?;
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let game_guard = acquire_snapshot_game_guard(
        disk_reconcile.inner(),
        &game_id,
        expected_identities.as_deref(),
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
    drop(game_guard);
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

    #[test]
    fn superseded_bulk_items_are_unprocessed_without_claiming_storage_success() {
        let result = bulk::BulkResult::new(vec!["E:/Mods/Bob".into()], Vec::new());
        let result = settle_superseded_bulk_items(result, 2, 1);
        assert_eq!(result.success, ["E:/Mods/Bob"]);
        assert!(result.cancelled);
        assert_eq!(result.processed_count, 1);
        assert_eq!(result.unprocessed_count, 1);
    }

    #[test]
    fn chunk_results_keep_all_commits_and_latest_disk_revision() {
        let mut aggregate = bulk::BulkResult::new(Vec::new(), Vec::new());
        let mut first = bulk::BulkResult::new(vec!["E:/Mods/A".into()], Vec::new());
        first.disk_revision = Some(3);
        let mut second = bulk::BulkResult::new(vec!["E:/Mods/B".into()], Vec::new());
        second.disk_revision = Some(5);
        merge_toggle_chunk_result(&mut aggregate, first);
        merge_toggle_chunk_result(&mut aggregate, second);
        assert_eq!(aggregate.success, ["E:/Mods/A", "E:/Mods/B"]);
        assert_eq!(aggregate.disk_revision, Some(5));
    }

    #[test]
    fn a_newer_single_intent_removes_only_its_folder_from_a_bulk_chunk() {
        let coordinator = MutationCoordinator::unconfigured();
        let paths = vec!["E:/Mods/A".into(), "E:/Mods/B".into()];
        let bulk = coordinator.admit_intents(
            "game",
            Some(1),
            paths.iter().cloned().map(IntentTarget::ModPath),
        );
        coordinator.admit_intents(
            "game",
            Some(2),
            [IntentTarget::ModPath("E:/Mods/DISABLED A".into())],
        );
        assert_eq!(current_toggle_paths(&bulk, "game", &paths), ["E:/Mods/B"]);
    }

    #[test]
    fn oversized_bulk_requests_are_rejected_before_registration_or_io() {
        let paths = vec!["mod".to_string(); MAX_BULK_PATHS + 1];
        assert!(matches!(
            validate_bulk_size(&paths),
            Err(AppError::Validation(_))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn snapshot_guard_rejects_a_replacement_that_occurs_while_waiting_for_the_game_lock() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let selected_path = temp_dir.path().join("Selected");
        let original_path = temp_dir.path().join("Original");
        std::fs::create_dir(&selected_path).expect("selected folder");
        let expected_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
            &selected_path,
        )
        .expect("filesystem identity");
        let expected = vec![(
            selected_path.to_string_lossy().into_owned(),
            expected_identity,
        )];
        let state = Arc::new(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new(),
        );
        let blocker = state.game_lock("game-1").lock_owned().await;
        let waiting_state = Arc::clone(&state);
        let waiter = tokio::spawn(async move {
            acquire_snapshot_game_guard(&waiting_state, "game-1", Some(&expected))
                .await
                .map(drop)
        });

        std::fs::rename(&selected_path, &original_path).expect("move original folder");
        std::fs::create_dir(&selected_path).expect("replacement folder");
        drop(blocker);

        let error = waiter
            .await
            .expect("guard task")
            .expect_err("replacement must expire snapshot");
        assert!(matches!(error, AppError::ExplorerSnapshotExpired));
    }

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
    fn bulk_cancel_targets_only_the_requested_operation() {
        let state = BulkCancelState::new();
        let first = state.register("first").expect("first operation");
        let second = state.register("second").expect("second operation");

        state.cancel("second");

        assert!(!first.flag().load(Ordering::SeqCst));
        assert!(second.flag().load(Ordering::SeqCst));
        drop(second);
        let replacement = state.register("second").expect("reused operation ID");
        assert!(!replacement.flag().load(Ordering::SeqCst));
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
