use crate::modules::ingestion::application::import_batch::types::{
    CommitImportBatchInput, ImportBatchReport, ImportDecision, ImportFlow, ImportItem,
    ImportItemStatus, SourceFingerprint,
};
use crate::modules::matching::application::deep_matcher::{EntryKind, MasterDb};
use crate::shared::errors::AppError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone)]
pub struct MoveJournalEntry {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl MoveJournalEntry {
    pub fn new(source: PathBuf, target: PathBuf) -> Self {
        Self { source, target }
    }
}

struct PlannedMove {
    item: ImportItem,
    source: PathBuf,
    object_dir: PathBuf,
    target: PathBuf,
    creates_object: bool,
}

pub fn fingerprint_matches(expected: &SourceFingerprint, current: &SourceFingerprint) -> bool {
    fingerprint_path_key(&expected.path) == fingerprint_path_key(&current.path)
        && expected.modified_unix_ms == current.modified_unix_ms
        && expected.size_bytes == current.size_bytes
        && expected.file_count == current.file_count
}

fn fingerprint_path_key(path: &str) -> String {
    let path = Path::new(path);
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    crate::shared::path_key::folder_path_key(&resolved.to_string_lossy(), None)
}

pub fn rollback_move_journal(journal: &[MoveJournalEntry]) -> Result<(), AppError> {
    let mut failures = Vec::new();
    for entry in journal.iter().rev() {
        if !entry.target.exists() {
            continue;
        }
        if entry.source.exists() {
            let cleanup = if entry.target.is_dir() {
                std::fs::remove_dir_all(&entry.target)
            } else {
                std::fs::remove_file(&entry.target)
            };
            if let Err(error) = cleanup {
                failures.push(format!(
                    "could not remove owned rollback target '{}': {error}",
                    entry.target.display()
                ));
            }
            continue;
        }
        if let Some(parent) = entry.source.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                failures.push(format!(
                    "could not recreate '{}': {error}",
                    parent.display()
                ));
                continue;
            }
        }
        if let Err(error) = crate::platform::fs::file_utils::rename_cross_drive_fallback(
            &entry.target,
            &entry.source,
        ) {
            failures.push(format!(
                "{} -> {}: {error}",
                entry.target.display(),
                entry.source.display()
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Import rollback was incomplete: {}",
            failures.join("; ")
        )))
    }
}

pub async fn commit_import_batch(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    input: CommitImportBatchInput,
    master_db: &MasterDb,
) -> Result<ImportBatchReport, AppError> {
    let mut batch =
        crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(pool, &input.batch_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", input.batch_id)))?;
    let requested_ids = validate_selected_items(&batch.items, &input.item_ids)?;
    let archive_pending_ids = batch
        .items
        .iter()
        .filter(|item| {
            requested_ids.contains(&item.id) && item.result.as_deref() == Some("archive_pending")
        })
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    if batch.flow == ImportFlow::ReadyToMove && !archive_pending_ids.is_empty() {
        return resume_ready_to_move_archive_finalization(app, pool, &batch).await;
    }
    let mods_root =
        crate::modules::games::adapters::sqlite::game::get_mod_path(pool, &batch.game_id)
            .await?
            .ok_or_else(|| AppError::Validation("Game has no configured mods path".to_string()))?;
    let canonical_root = Path::new(&mods_root).canonicalize().map_err(|error| {
        AppError::Validation(format!("Configured mods path is unavailable: {error}"))
    })?;
    if batch
        .items
        .iter()
        .any(|item| requested_ids.contains(&item.id) && item.status == ImportItemStatus::Committing)
    {
        recover_interrupted_commits(
            app,
            pool,
            &batch,
            &requested_ids,
            master_db,
            &canonical_root,
        )
        .await?;
        batch = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
            pool,
            &input.batch_id,
        )
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", input.batch_id)))?;
    }
    let recovery_ids = batch
        .items
        .iter()
        .filter(|item| {
            requested_ids.contains(&item.id)
                && matches!(
                    item.status,
                    ImportItemStatus::Partial
                        | ImportItemStatus::Reconciling
                        | ImportItemStatus::MetadataPending
                        | ImportItemStatus::FinalizingMetadata
                )
        })
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    if !recovery_ids.is_empty() {
        return resume_committed_items(
            app,
            pool,
            &batch,
            &recovery_ids,
            master_db,
            &mods_root,
            &canonical_root,
        )
        .await;
    }
    let recovered_collisions = batch
        .items
        .iter()
        .filter(|item| {
            requested_ids.contains(&item.id) && item.result.as_deref() == Some("collision")
        })
        .count() as u32;
    let recovered_failures = batch
        .items
        .iter()
        .filter(|item| requested_ids.contains(&item.id) && item.status == ImportItemStatus::Failed)
        .count() as u32;
    let selected_ids = batch
        .items
        .iter()
        .filter(|item| requested_ids.contains(&item.id) && item.status == ImportItemStatus::Ready)
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    if selected_ids.is_empty() {
        crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
            pool, &batch.id,
        )
        .await?;
        return Ok(ImportBatchReport {
            batch_id: batch.id,
            moved: 0,
            reallocated: 0,
            created_canonical_folders: 0,
            skipped: recovered_collisions,
            collisions: recovered_collisions,
            metadata_pending: 0,
            failed: recovered_failures,
        });
    }
    let mut plans = Vec::with_capacity(selected_ids.len());
    for item in batch
        .items
        .iter()
        .filter(|item| selected_ids.contains(&item.id))
    {
        plans.push(resolve_plan(pool, &batch, item, master_db, &canonical_root).await?);
    }
    validate_unique_targets(&plans)?;
    for plan in &plans {
        validate_preview_fingerprint(plan)?;
    }

    let preflight_paths = plans
        .iter()
        .filter(|plan| !plan.target.exists())
        .map(|plan| plan.target.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if !preflight_paths.is_empty() {
        crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
            app,
            pool,
            &batch.game_id,
            Some(&preflight_paths),
        )
        .await?;
    }
    let operation_lock = app
        .try_state::<crate::modules::mutation::coordinator::MutationCoordinator>()
        .ok_or_else(|| {
            AppError::Internal("MutationCoordinator state is unavailable".to_string())
        })?;
    let disk_reconcile_state = app
        .try_state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
        .ok_or_else(|| AppError::Internal("DiskReconcileState is unavailable".to_string()))?;
    let game_guard = disk_reconcile_state
        .game_lock(&batch.game_id)
        .lock_owned()
        .await;
    let operation_guard = operation_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "import-commit",
            batch.game_id.clone(),
            plans
                .iter()
                .enumerate()
                .map(|(sequence, plan)| {
                    crate::modules::mutation::api::PlannedStep::rename(
                        sequence as u32,
                        plan.source.clone(),
                        plan.target.clone(),
                    )
                })
                .collect(),
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let target_manifest_index = app
        .try_state::<crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState>()
        .ok_or_else(|| AppError::Internal("TargetManifestIndexState is unavailable".to_string()))?;
    if let Err(error) = validate_targets_still_available(
        &plans,
        &canonical_root,
        &batch.game_id,
        target_manifest_index.inner(),
    ) {
        for sequence in 0..plans.len() {
            mutation_lease.mark_step_rolled_back(sequence as u32)?;
        }
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Err(error);
    }
    let selected_id_list = selected_ids.iter().cloned().collect::<Vec<_>>();
    if !crate::modules::ingestion::adapters::sqlite::import_batch::begin_batch_commit(
        pool,
        &batch.id,
        &selected_id_list,
    )
    .await?
    {
        for sequence in 0..plans.len() {
            mutation_lease.mark_step_rolled_back(sequence as u32)?;
        }
        mutation_lease.begin_rollback()?;
        mutation_lease.finish_rollback()?;
        return Err(AppError::Validation(
            "Import batch changed after preview; refresh it before committing".to_string(),
        ));
    }
    let watcher = app
        .try_state::<crate::modules::workspace::application::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState is unavailable".to_string()))?;
    let suppression = watcher.suppressor.suppress_paths(
        plans
            .iter()
            .flat_map(|plan| [plan.source.as_path(), plan.target.as_path()]),
    );
    let mut journal = Vec::new();
    let mut created_directories = Vec::new();
    let mut collision_ids = BTreeSet::new();
    for plan in &plans {
        crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
            pool,
            &plan.item.id,
            ImportItemStatus::Committing,
            Some(&plan.target.to_string_lossy()),
            Some("planned"),
            None,
        )
        .await?;
    }
    let mutation_result = execute_moves(
        pool,
        &plans,
        &mut journal,
        &mut created_directories,
        &mut collision_ids,
    )
    .await;

    if let Err(move_error) = mutation_result {
        let rollback = rollback_move_journal(&journal);
        let rollback_succeeded = rollback.is_ok();
        cleanup_created_directories(&created_directories);
        for plan in &plans {
            if rollback_succeeded {
                crate::modules::ingestion::adapters::sqlite::import_batch::restore_item_after_rollback(
                    pool,
                    &plan.item.id,
                    &move_error.to_string(),
                )
                .await?;
            } else {
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &plan.item.id,
                    ImportItemStatus::Failed,
                    None,
                    None,
                    Some(&move_error.to_string()),
                )
                .await?;
            }
        }
        crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
            pool, &batch.id,
        )
        .await?;
        drop(suppression);
        if rollback_succeeded {
            for sequence in 0..plans.len() {
                mutation_lease.mark_step_rolled_back(sequence as u32)?;
            }
            mutation_lease.begin_rollback()?;
        }
        let recovery =
            crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                app,
                pool,
                &batch.game_id,
                &mutation_lease,
            )
            .await;
        return match (rollback, recovery) {
            (Ok(()), Ok(_)) => {
                mutation_lease.finish_rollback()?;
                Err(move_error)
            }
            (rollback, recovery) => {
                let combined = format!(
                    "Import move failed: {move_error}; rollback: {}; recovery reconcile: {}",
                    result_label(rollback),
                    result_label(recovery)
                );
                mutation_lease.fail(combined.clone())?;
                Err(AppError::Internal(combined))
            }
        };
    }

    for (sequence, plan) in plans.iter().enumerate() {
        if collision_ids.contains(&plan.item.id) {
            mutation_lease.mark_step_skipped(sequence as u32)?;
        } else {
            mutation_lease.mark_step_applied(sequence as u32)?;
        }
    }

    for plan in &plans {
        if collision_ids.contains(&plan.item.id) {
            continue;
        }
        crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
            pool,
            &plan.item.id,
            ImportItemStatus::Reconciling,
            Some(&plan.target.to_string_lossy()),
            Some("moved"),
            None,
        )
        .await?;
    }
    drop(suppression);

    let changed_paths = journal
        .iter()
        .flat_map(|entry| {
            [
                entry.source.to_string_lossy().into_owned(),
                entry.target.to_string_lossy().into_owned(),
            ]
        })
        .collect::<Vec<_>>();
    let path_hints = plans
        .iter()
        .filter(|plan| !collision_ids.contains(&plan.item.id))
        .filter_map(|plan| {
            plan.item.destination_object_id.as_ref().map(|object_id| {
                crate::modules::library::application::mods::organizer_move::OrganizerMovePathHint {
                    old_path: plan.source.to_string_lossy().into_owned(),
                    new_path: plan.target.to_string_lossy().into_owned(),
                    target_object_id: object_id.clone(),
                }
            })
        })
        .collect();
    let reconcile =
        crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            app,
            pool,
            &batch.game_id,
            changed_paths,
            path_hints,
            &mutation_lease,
        )
        .await;
    let reconcile = match reconcile {
        Ok(result) if result.status.applied() => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            let _ = app.emit("disk_reconcile:result", &result);
            result
        }
        Ok(result) => {
            let error = AppError::Io(format!(
                "Disk reconcile requires attention after import: {:?}",
                result.status
            ));
            if let Err(rollback_error) = rollback_import_after_reconcile_failure(
                app,
                pool,
                &batch,
                &plans,
                &collision_ids,
                &journal,
                &created_directories,
                &mutation_lease,
                &error,
            )
            .await
            {
                mutation_lease.fail(rollback_error.to_string())?;
                return Err(rollback_error);
            }
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
        Err(error) => {
            if let Err(rollback_error) = rollback_import_after_reconcile_failure(
                app,
                pool,
                &batch,
                &plans,
                &collision_ids,
                &journal,
                &created_directories,
                &mutation_lease,
                &error,
            )
            .await
            {
                mutation_lease.fail(rollback_error.to_string())?;
                return Err(rollback_error);
            }
            mutation_lease.finish_rollback()?;
            return Err(error);
        }
    };
    let _ = reconcile;

    let mut aliases_changed = false;
    let mut metadata_pending = 0_u32;
    for plan in &plans {
        if collision_ids.contains(&plan.item.id) {
            continue;
        }
        crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
            pool,
            &plan.item.id,
            ImportItemStatus::FinalizingMetadata,
            Some(&plan.target.to_string_lossy()),
            Some("reconciled"),
            None,
        )
        .await?;
        let projection = async {
            let object_id =
                resolve_reconciled_object_id(pool, &batch.game_id, &mods_root, plan).await?;
            let mod_id = resolve_reconciled_mod_id(pool, &batch.game_id, &plan.target).await?;
            crate::modules::ingestion::adapters::sqlite::import_batch::bind_reconciled_destination(
                pool,
                &plan.item.id,
                &object_id,
                &mod_id,
            )
            .await?;
            Ok::<String, AppError>(object_id)
        }
        .await;
        let object_id = match projection {
            Ok(object_id) => object_id,
            Err(error) => {
                metadata_pending += 1;
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &plan.item.id,
                    ImportItemStatus::MetadataPending,
                    Some(&plan.target.to_string_lossy()),
                    Some("metadata_pending"),
                    Some(&error.to_string()),
                )
                .await?;
                continue;
            }
        };
        let classification =
            apply_item_classification(pool, &batch.game_id, &plan.item, object_id).await;
        match classification {
            Ok(result) => {
                aliases_changed |= result.aliases_changed;
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &plan.item.id,
                    ImportItemStatus::Done,
                    Some(&plan.target.to_string_lossy()),
                    Some("done"),
                    None,
                )
                .await?;
            }
            Err(error) => {
                metadata_pending += 1;
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &plan.item.id,
                    ImportItemStatus::MetadataPending,
                    Some(&plan.target.to_string_lossy()),
                    Some("metadata_pending"),
                    Some(&error.to_string()),
                )
                .await?;
            }
        }
    }
    if aliases_changed {
        crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(app)
            .await;
    }
    settle_runtime_effects(app, pool, &batch.game_id).await;
    let archive_failed = finalize_batch_after_metadata(app, pool, &batch).await?;
    let mut report = report_for(&batch.id, &plans, &collision_ids, false);
    report.skipped += recovered_collisions;
    report.collisions += recovered_collisions;
    report.failed += recovered_failures;
    report.failed += u32::from(archive_failed);
    report.metadata_pending = metadata_pending;
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
async fn rollback_import_after_reconcile_failure(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
    plans: &[PlannedMove],
    collision_ids: &BTreeSet<String>,
    journal: &[MoveJournalEntry],
    created_directories: &[PathBuf],
    mutation_lease: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    reconcile_error: &AppError,
) -> Result<(), AppError> {
    mutation_lease.begin_rollback()?;
    if let Err(rollback_error) = rollback_move_journal(journal) {
        let combined = format!("{reconcile_error}; import rollback failed: {rollback_error}");
        return Err(AppError::Io(combined));
    }
    cleanup_created_directories(created_directories);
    for (sequence, plan) in plans.iter().enumerate() {
        if !collision_ids.contains(&plan.item.id) {
            mutation_lease.mark_step_rolled_back(sequence as u32)?;
        }
        crate::modules::ingestion::adapters::sqlite::import_batch::restore_item_after_rollback(
            pool,
            &plan.item.id,
            &reconcile_error.to_string(),
        )
        .await?;
    }
    crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
        pool, &batch.id,
    )
    .await?;
    crate::modules::reconciliation::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
        app,
        pool,
        &batch.game_id,
        mutation_lease,
    )
    .await
    .map_err(|rollback_reconcile_error| {
        AppError::Io(format!(
            "{reconcile_error}; rollback projection failed: {rollback_reconcile_error}"
        ))
    })?;
    Ok(())
}

async fn apply_item_classification(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    item: &ImportItem,
    object_id: String,
) -> Result<
    crate::modules::catalog::application::objects::classification::ClassificationWriteResult,
    AppError,
> {
    crate::modules::catalog::application::objects::classification::apply_object_classification(
        pool,
        crate::modules::catalog::application::objects::classification::ObjectClassificationInput {
            game_id: game_id.to_string(),
            object_id,
            category: item
                .match_category
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "Import item '{}' lost its category decision",
                        item.id
                    ))
                })?
                .as_str()
                .to_string(),
            subcategory: item.sub_category.clone(),
            metadata: item.classification_metadata.clone(),
            canonical_match: item.selected_entry_key.as_ref().map(|entry_key| {
                crate::modules::catalog::application::objects::classification::CanonicalClassificationMatch {
                    entry_key: entry_key.clone(),
                    alias_name: item.selected_alias_name.clone(),
                    confidence: Some(f64::from(item.confidence_percentage) / 100.0),
                    reason: Some(
                        serde_json::to_string(&item.evidence).unwrap_or_else(|_| "[]".to_string()),
                    ),
                    source: "match_wizard".to_string(),
                }
            }),
            confirmed_source_alias: item.selected_entry_key.as_ref().map(|_| {
                crate::modules::catalog::application::match_engine::inspection::source_display_name(&item.planned_name)
            }),
        },
    )
    .await
}

async fn recover_interrupted_commits(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
    requested_ids: &BTreeSet<String>,
    master_db: &MasterDb,
    mods_root: &Path,
) -> Result<(), AppError> {
    let interrupted = batch
        .items
        .iter()
        .filter(|item| {
            requested_ids.contains(&item.id) && item.status == ImportItemStatus::Committing
        })
        .collect::<Vec<_>>();
    if interrupted.is_empty() {
        return Ok(());
    }
    let mut paths = Vec::with_capacity(interrupted.len());
    for item in &interrupted {
        let source = PathBuf::from(
            item.staging_path
                .as_deref()
                .unwrap_or(item.source_path.as_str()),
        );
        let (object_dir, _) = resolve_object_dir(pool, batch, item, master_db, mods_root).await?;
        let target = append_target_subpath(&object_dir, batch.target_subpath.as_deref())?
            .join(physical_import_name(&item.planned_name));
        ensure_target_under_root(mods_root, &target, true)?;
        paths.push(((*item).clone(), source, target));
    }

    let preflight_paths = paths
        .iter()
        .filter(|(_, _, target)| !target.exists())
        .map(|(_, _, target)| target.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if !preflight_paths.is_empty() {
        crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
            app,
            pool,
            &batch.game_id,
            Some(&preflight_paths),
        )
        .await?;
    }
    let operation_lock = app
        .try_state::<crate::modules::mutation::coordinator::MutationCoordinator>()
        .ok_or_else(|| {
            AppError::Internal("MutationCoordinator state is unavailable".to_string())
        })?;
    let disk_state = app
        .try_state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
        .ok_or_else(|| AppError::Internal("DiskReconcileState is unavailable".to_string()))?;
    let lease = disk_state
        .acquire_mutation_lease(&batch.game_id, operation_lock.inner_lock())
        .await?;
    let watcher = app
        .try_state::<crate::modules::workspace::application::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState is unavailable".to_string()))?;
    let suppression = watcher.suppressor.suppress_paths(
        paths
            .iter()
            .flat_map(|(_, source, target)| [source.as_path(), target.as_path()]),
    );
    let mut changed_paths = Vec::new();
    for (item, source, target) in &paths {
        match (source.exists(), target.exists()) {
            (true, false) => {
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::Ready,
                    None,
                    Some("recovered_ready"),
                    None,
                )
                .await?;
            }
            (false, true) => {
                if let Some(parent) = source.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                crate::platform::fs::file_utils::rename_cross_drive_fallback(target, source)?;
                changed_paths.extend([
                    source.to_string_lossy().into_owned(),
                    target.to_string_lossy().into_owned(),
                ]);
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::Ready,
                    None,
                    Some("recovered_ready"),
                    None,
                )
                .await?;
            }
            (true, true) => {
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::Skipped,
                    None,
                    Some("collision"),
                    Some("Destination appeared while the interrupted import was recovering"),
                )
                .await?;
            }
            (false, false) => {
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::Failed,
                    None,
                    Some("recovery_failed"),
                    Some("Neither the import source nor its planned destination exists"),
                )
                .await?;
            }
        }
    }
    drop(suppression);
    crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
        pool, &batch.id,
    )
    .await?;
    if !changed_paths.is_empty() {
        crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            app,
            pool,
            &batch.game_id,
            changed_paths,
            Vec::new(),
            &lease,
        )
        .await?;
    }
    drop(lease);
    Ok(())
}

async fn resume_committed_items(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
    item_ids: &BTreeSet<String>,
    master_db: &MasterDb,
    mods_root: &str,
    canonical_root: &Path,
) -> Result<ImportBatchReport, AppError> {
    let items = batch
        .items
        .iter()
        .filter(|item| item_ids.contains(&item.id))
        .cloned()
        .collect::<Vec<_>>();
    let needs_reconcile = items.iter().any(|item| {
        matches!(
            item.status,
            ImportItemStatus::Partial | ImportItemStatus::Reconciling
        )
    });
    let mut lease = None;
    if needs_reconcile {
        crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight(app, pool, &batch.game_id)
            .await?;
        let operation_lock = app
            .try_state::<crate::modules::mutation::coordinator::MutationCoordinator>()
            .ok_or_else(|| {
                AppError::Internal("MutationCoordinator state is unavailable".to_string())
            })?;
        let disk_state = app
            .try_state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
            .ok_or_else(|| AppError::Internal("DiskReconcileState is unavailable".to_string()))?;
        lease = Some(
            disk_state
                .acquire_mutation_lease(&batch.game_id, operation_lock.inner_lock())
                .await?,
        );
    }
    let ids = item_ids.iter().cloned().collect::<Vec<_>>();
    if !crate::modules::ingestion::adapters::sqlite::import_batch::prepare_items_for_recovery(
        pool, &batch.id, &ids,
    )
    .await?
    {
        return Err(AppError::Validation(
            "Import recovery state changed; reload the batch before retrying".to_string(),
        ));
    }

    if let Some(active_lease) = lease.as_ref() {
        let changed_paths = items
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    ImportItemStatus::Partial | ImportItemStatus::Reconciling
                )
            })
            .flat_map(|item| {
                [
                    item.source_path.clone(),
                    item.destination_path.clone().unwrap_or_default(),
                ]
            })
            .filter(|path| !path.is_empty())
            .collect::<Vec<_>>();
        let hints = items
            .iter()
            .filter_map(|item| {
                item.destination_object_id.as_ref().and_then(|object_id| {
                    item.destination_path.as_ref().map(|destination| {
                        crate::modules::library::application::mods::organizer_move::OrganizerMovePathHint {
                            old_path: item.source_path.clone(),
                            new_path: destination.clone(),
                            target_object_id: object_id.clone(),
                        }
                    })
                })
            })
            .collect();
        let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            app,
            pool,
            &batch.game_id,
            changed_paths,
            hints,
            active_lease,
        )
        .await;
        match reconcile {
            Ok(result) if result.status.applied() => {
                let _ = app.emit("disk_reconcile:result", &result);
            }
            Ok(result) => {
                let warning = format!(
                    "Disk reconcile still requires attention: {:?}",
                    result.status
                );
                for item in &items {
                    crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                        pool,
                        &item.id,
                        ImportItemStatus::Partial,
                        item.destination_path.as_deref(),
                        Some("committed_with_warning"),
                        Some(&warning),
                    )
                    .await?;
                }
                crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
                    pool, &batch.id,
                )
                .await?;
                return Ok(recovery_report(&batch.id, 0, 1));
            }
            Err(error) => {
                let warning = format!("Disk reconcile retry failed: {error}");
                for item in &items {
                    crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                        pool,
                        &item.id,
                        ImportItemStatus::Partial,
                        item.destination_path.as_deref(),
                        Some("committed_with_warning"),
                        Some(&warning),
                    )
                    .await?;
                }
                crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
                    pool, &batch.id,
                )
                .await?;
                return Ok(recovery_report(&batch.id, 0, 1));
            }
        }
    }
    drop(lease);

    let mut aliases_changed = false;
    let mut metadata_pending = 0_u32;
    for item in &items {
        crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
            pool,
            &item.id,
            ImportItemStatus::FinalizingMetadata,
            item.destination_path.as_deref(),
            Some("reconciled"),
            None,
        )
        .await?;
        let projection = async {
            let object_id =
                resolve_recovery_object_id(pool, batch, item, master_db, mods_root, canonical_root)
                    .await?;
            let destination_path = item.destination_path.as_deref().ok_or_else(|| {
                AppError::Internal(format!(
                    "Import item '{}' has no reconciled destination path",
                    item.id
                ))
            })?;
            let mod_id =
                resolve_reconciled_mod_id(pool, &batch.game_id, Path::new(destination_path))
                    .await?;
            crate::modules::ingestion::adapters::sqlite::import_batch::bind_reconciled_destination(
                pool, &item.id, &object_id, &mod_id,
            )
            .await?;
            Ok::<String, AppError>(object_id)
        }
        .await;
        let object_id = match projection {
            Ok(object_id) => object_id,
            Err(error) => {
                metadata_pending += 1;
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::MetadataPending,
                    item.destination_path.as_deref(),
                    Some("metadata_pending"),
                    Some(&error.to_string()),
                )
                .await?;
                continue;
            }
        };
        match apply_item_classification(pool, &batch.game_id, item, object_id).await {
            Ok(result) => {
                aliases_changed |= result.aliases_changed;
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::Done,
                    item.destination_path.as_deref(),
                    Some("done"),
                    None,
                )
                .await?;
            }
            Err(error) => {
                metadata_pending += 1;
                crate::modules::ingestion::adapters::sqlite::import_batch::set_commit_item_state(
                    pool,
                    &item.id,
                    ImportItemStatus::MetadataPending,
                    item.destination_path.as_deref(),
                    Some("metadata_pending"),
                    Some(&error.to_string()),
                )
                .await?;
            }
        }
    }
    if aliases_changed {
        crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(app)
            .await;
    }
    settle_runtime_effects(app, pool, &batch.game_id).await;
    let archive_failed = finalize_batch_after_metadata(app, pool, batch).await?;
    Ok(recovery_report(
        &batch.id,
        metadata_pending,
        u32::from(archive_failed),
    ))
}

async fn resolve_recovery_object_id(
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
    item: &ImportItem,
    master_db: &MasterDb,
    mods_root: &str,
    canonical_root: &Path,
) -> Result<String, AppError> {
    if let Some(object_id) = &item.destination_object_id {
        return Ok(object_id.clone());
    }
    let (object_dir, _) = resolve_object_dir(pool, batch, item, master_db, canonical_root).await?;
    let key =
        crate::shared::path_key::folder_path_key(&object_dir.to_string_lossy(), Some(mods_root));
    let mut connection = pool.acquire().await?;
    crate::modules::catalog::adapters::sqlite::object::get_object_id_by_folder_key(
        &mut connection,
        &batch.game_id,
        &key,
    )
    .await?
    .ok_or_else(|| {
        AppError::Internal(format!(
            "Disk reconcile did not create an object for '{}'",
            object_dir.display()
        ))
    })
}

fn recovery_report(batch_id: &str, metadata_pending: u32, failed: u32) -> ImportBatchReport {
    ImportBatchReport {
        batch_id: batch_id.to_string(),
        moved: 0,
        reallocated: 0,
        created_canonical_folders: 0,
        skipped: 0,
        collisions: 0,
        metadata_pending,
        failed,
    }
}

fn cleanup_import_staging(app: &tauri::AppHandle, batch_id: &str) -> Result<(), AppError> {
    let root = app.path().app_data_dir()?.join("import-staging");
    crate::modules::ingestion::application::import_batch::staging::cleanup_batch_staging(
        &root, batch_id,
    )?;
    Ok(())
}

fn physical_import_name(planned_name: &str) -> String {
    crate::modules::library::application::mods::core_ops::standardize_prefix(planned_name, false)
}

fn validate_selected_items(
    items: &[ImportItem],
    item_ids: &[String],
) -> Result<BTreeSet<String>, AppError> {
    if item_ids.is_empty() {
        return Err(AppError::Validation(
            "Select at least one ready import item".to_string(),
        ));
    }
    let selected = item_ids.iter().cloned().collect::<BTreeSet<_>>();
    if selected.len() != item_ids.len() {
        return Err(AppError::Validation(
            "Import item IDs must be unique".to_string(),
        ));
    }
    for item_id in &selected {
        let Some(item) = items.iter().find(|item| &item.id == item_id) else {
            return Err(AppError::Validation(format!(
                "Import item '{item_id}' does not belong to this batch"
            )));
        };
        if !matches!(
            item.status,
            ImportItemStatus::Ready
                | ImportItemStatus::Committing
                | ImportItemStatus::Reconciling
                | ImportItemStatus::FinalizingMetadata
                | ImportItemStatus::Partial
                | ImportItemStatus::MetadataPending
        ) {
            return Err(AppError::Validation(format!(
                "Import item '{item_id}' is neither ready nor resumable"
            )));
        }
    }
    Ok(selected)
}

async fn resolve_plan(
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
    item: &ImportItem,
    master_db: &MasterDb,
    mods_root: &Path,
) -> Result<PlannedMove, AppError> {
    let source = Path::new(
        item.staging_path
            .as_deref()
            .unwrap_or(item.source_path.as_str()),
    )
    .canonicalize()
    .map_err(|error| {
        AppError::Validation(format!(
            "stale_preview: source '{}' is unavailable: {error}",
            item.source_path
        ))
    })?;
    let physical_name = physical_import_name(&item.planned_name);
    crate::modules::library::application::mods::core_ops::validate_folder_name_component(
        &physical_name,
    )?;
    let (object_dir, creates_object) =
        resolve_object_dir(pool, batch, item, master_db, mods_root).await?;
    ensure_target_under_root(mods_root, &object_dir, creates_object)?;
    let placement_dir = append_target_subpath(&object_dir, batch.target_subpath.as_deref())?;
    let target = placement_dir.join(physical_name);
    ensure_target_under_root(mods_root, &target, true)?;
    Ok(PlannedMove {
        item: item.clone(),
        source,
        object_dir,
        target,
        creates_object,
    })
}

async fn resolve_object_dir(
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
    item: &ImportItem,
    master_db: &MasterDb,
    mods_root: &Path,
) -> Result<(PathBuf, bool), AppError> {
    if let Some(object_id) = &item.destination_object_id {
        if item.decision == ImportDecision::KeepSpecificTarget
            && batch.target_object_id.as_ref() != Some(object_id)
        {
            return Err(AppError::Validation(
                "Specific-target confirmation no longer points at its owning object".to_string(),
            ));
        }
        let (game_id, folder_path) =
            crate::modules::catalog::adapters::sqlite::object::get_game_id_and_folder_path(
                pool, object_id,
            )
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Destination object '{object_id}'")))?;
        if game_id != batch.game_id {
            return Err(AppError::Security(
                "Destination object belongs to a different game".to_string(),
            ));
        }
        let folder_path = folder_path.ok_or_else(|| {
            AppError::Validation("Destination object has no folder path".to_string())
        })?;
        let raw = PathBuf::from(folder_path);
        let resolved = if raw.is_absolute() {
            raw
        } else {
            mods_root.join(raw)
        };
        let canonical = resolved.canonicalize().map_err(|error| {
            AppError::Validation(format!("Destination object is unavailable: {error}"))
        })?;
        Ok((canonical, false))
    } else {
        let entry_key = item.selected_entry_key.as_deref().ok_or_else(|| {
            AppError::Validation("Create-folder decision has no canonical entry".to_string())
        })?;
        let entry = master_db
            .entries
            .iter()
            .find(|entry| {
                entry.entry_kind == EntryKind::Canonical
                    && crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(&entry.name)
                        == entry_key
            })
            .ok_or_else(|| {
                AppError::Validation(
                    "Create folder is allowed only for a canonical MasterDB entry".to_string(),
                )
            })?;
        if item.match_category.map(|category| category.as_str()) != Some(entry.object_type.as_str())
        {
            return Err(AppError::Validation(
                "Canonical entry no longer matches the confirmed category".to_string(),
            ));
        }
        crate::modules::library::application::mods::core_ops::validate_folder_name_component(
            &entry.name,
        )?;
        Ok((mods_root.join(&entry.name), true))
    }
}

fn append_target_subpath(root: &Path, subpath: Option<&str>) -> Result<PathBuf, AppError> {
    let Some(subpath) = subpath.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(root.to_path_buf());
    };
    let relative = Path::new(subpath);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::Security(
            "Import target subpath must contain only normal relative path segments".to_string(),
        ));
    }
    Ok(root.join(relative))
}

fn ensure_target_under_root(
    root: &Path,
    target: &Path,
    may_not_exist: bool,
) -> Result<(), AppError> {
    let checked = if target.exists() {
        target.canonicalize()?
    } else if may_not_exist {
        let mut ancestor = target.parent();
        while let Some(path) = ancestor {
            if path.exists() {
                let canonical = path.canonicalize()?;
                if !canonical.starts_with(root) {
                    return Err(AppError::Security(format!(
                        "Import destination escapes the configured mods path: {}",
                        target.display()
                    )));
                }
                return Ok(());
            }
            ancestor = path.parent();
        }
        return Err(AppError::Security(
            "Import destination has no existing parent".to_string(),
        ));
    } else {
        return Err(AppError::Validation(format!(
            "Import destination is unavailable: {}",
            target.display()
        )));
    };
    if checked.starts_with(root) {
        Ok(())
    } else {
        Err(AppError::Security(format!(
            "Import destination escapes the configured mods path: {}",
            target.display()
        )))
    }
}

fn validate_unique_targets(plans: &[PlannedMove]) -> Result<(), AppError> {
    let mut targets = BTreeSet::new();
    for plan in plans {
        let key = crate::shared::path_key::folder_path_key(&plan.target.to_string_lossy(), None);
        if !targets.insert(key) {
            return Err(AppError::Validation(format!(
                "Multiple import items resolve to the same destination: {}",
                plan.target.display()
            )));
        }
    }
    Ok(())
}

fn validate_preview_fingerprint(plan: &PlannedMove) -> Result<(), AppError> {
    let expected = plan.item.fingerprint.as_ref().ok_or_else(|| {
        AppError::Validation(format!(
            "stale_preview: import item '{}' has no source fingerprint",
            plan.item.id
        ))
    })?;
    let current = crate::modules::catalog::application::match_engine::inspection::inspect_source(
        &crate::modules::catalog::application::match_engine::inspection::InspectionRequest {
            source_path: plan.source.clone(),
            planned_name: Some(plan.item.planned_name.clone()),
            match_extensions: vec![
                "ini".to_string(),
                "dds".to_string(),
                "buf".to_string(),
                "ib".to_string(),
            ],
        },
    )?
    .fingerprint;
    if fingerprint_matches(expected, &current) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "stale_preview: source changed after analysis for item '{}'",
            plan.item.id
        )))
    }
}

fn validate_targets_still_available(
    plans: &[PlannedMove],
    mods_root: &Path,
    game_id: &str,
    target_manifest_index: &crate::modules::ingestion::application::import_batch::target_manifest_index::TargetManifestIndexState,
) -> Result<(), AppError> {
    for plan in plans {
        let expected_manifest = plan.item.payload_manifest.as_ref().ok_or_else(|| {
            AppError::Validation(format!(
                "stale_preview: import item '{}' has no payload manifest",
                plan.item.id
            ))
        })?;
        let current_manifest = crate::modules::ingestion::application::import_batch::payload_manifest::build_validated_import_payload_manifest(
            &plan.source,
            None,
        )?;
        if current_manifest.version != expected_manifest.version
            || current_manifest.content_sha256 != expected_manifest.content_sha256
        {
            return Err(AppError::Validation(format!(
                "stale_preview: payload changed after analysis for item '{}'",
                plan.item.id
            )));
        }
        if let Some(existing) = target_manifest_index.find_existing_payload_match(
            &plan.item.batch_id,
            game_id,
            mods_root,
            &current_manifest,
        )? {
            return Err(AppError::Validation(format!(
                "target_changed: an identical payload is now installed at '{}'; refresh this item before committing",
                existing.display()
            )));
        }
        let Some(parent) = plan.target.parent() else {
            return Err(AppError::Validation(
                "Import destination has no parent".to_string(),
            ));
        };
        if !parent.exists() {
            continue;
        }
        let Some(target_name) = plan.target.file_name().and_then(|name| name.to_str()) else {
            return Err(AppError::Validation(format!(
                "Import destination has an invalid folder name: {}",
                plan.target.display()
            )));
        };
        if let Some(existing) =
            crate::modules::library::application::mods::core_ops::find_sibling_identity_collision(
                parent,
                target_name,
                None,
            )
        {
            return Err(AppError::Validation(format!(
                "target_changed: destination '{}' is now occupied by '{}'; refresh this item before committing",
                plan.target.display(),
                existing.display()
            )));
        }
    }
    Ok(())
}

async fn execute_moves(
    _pool: &sqlx::SqlitePool,
    plans: &[PlannedMove],
    journal: &mut Vec<MoveJournalEntry>,
    created_directories: &mut Vec<PathBuf>,
    _collision_ids: &mut BTreeSet<String>,
) -> Result<(), AppError> {
    for plan in plans {
        if plan.target.exists() {
            return Err(AppError::Validation(format!(
                "target_changed: destination '{}' became occupied during commit",
                plan.target.display()
            )));
        }
        if !plan.object_dir.exists() {
            std::fs::create_dir(&plan.object_dir)?;
            created_directories.push(plan.object_dir.clone());
        }
        let parent = plan
            .target
            .parent()
            .ok_or_else(|| AppError::Validation("Import destination has no parent".to_string()))?;
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            created_directories.push(parent.to_path_buf());
        }
        match crate::platform::fs::file_utils::rename_cross_drive_fallback_tracked(
            &plan.source,
            &plan.target,
        ) {
            Ok(()) => journal.push(MoveJournalEntry::new(
                plan.source.clone(),
                plan.target.clone(),
            )),
            Err(error) => {
                if error.target_is_owned() {
                    journal.push(MoveJournalEntry::new(
                        plan.source.clone(),
                        plan.target.clone(),
                    ));
                }
                return Err(error.into_io_error().into());
            }
        }
    }
    Ok(())
}

fn cleanup_created_directories(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        if path.is_dir() {
            if let Err(error) = std::fs::remove_dir(path) {
                log::warn!(
                    "Could not remove empty import rollback directory '{}': {error}",
                    path.display()
                );
            }
        }
    }
}

async fn resolve_reconciled_object_id(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_root: &str,
    plan: &PlannedMove,
) -> Result<String, AppError> {
    if let Some(object_id) = &plan.item.destination_object_id {
        return Ok(object_id.clone());
    }
    let key = crate::shared::path_key::folder_path_key(
        &plan.object_dir.to_string_lossy(),
        Some(mods_root),
    );
    let mut connection = pool.acquire().await?;
    crate::modules::catalog::adapters::sqlite::object::get_object_id_by_folder_key(
        &mut connection,
        game_id,
        &key,
    )
    .await?
    .ok_or_else(|| {
        AppError::Internal(format!(
            "Disk reconcile did not create an object for '{}'",
            plan.object_dir.display()
        ))
    })
}

async fn resolve_reconciled_mod_id(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    destination: &Path,
) -> Result<String, AppError> {
    crate::modules::library::adapters::sqlite::mods::get_mod_id_and_object_id_by_path(
        pool,
        &destination.to_string_lossy(),
        game_id,
    )
    .await?
    .map(|(mod_id, _)| mod_id)
    .ok_or_else(|| {
        AppError::Internal(format!(
            "Disk reconcile did not create a mod for '{}'",
            destination.display()
        ))
    })
}

async fn settle_runtime_effects(app: &tauri::AppHandle, pool: &sqlx::SqlitePool, game_id: &str) {
    let Some(config) =
        app.try_state::<crate::modules::settings::application::config::ConfigService>()
    else {
        log::warn!("Classification completed but ConfigService is unavailable");
        return;
    };
    let Some(state) =
        app.try_state::<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState>()
    else {
        log::warn!("Classification completed but DiskReconcileState is unavailable");
        return;
    };
    let settlement = crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
        state.inner(),
        crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
            pool,
            config: config.inner(),
            game_id,
            collections_dirty: true,
            overlay_refresh: true,
            overlay_cause: crate::modules::system::application::app::post_apply::OverlaySyncCause::EffectiveModsChanged,
        },
    )
    .await;
    if let Some(warning) = settlement.warning {
        log::warn!("Import runtime effects pending: {warning}");
    }
}

pub(super) async fn finalize_ready_to_move_archives(
    pool: &sqlx::SqlitePool,
    batch_id: &str,
) -> Result<(), AppError> {
    let Some(batch) =
        crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(pool, batch_id)
            .await?
    else {
        return Ok(());
    };
    let mut groups = BTreeMap::<String, Vec<&ImportItem>>::new();
    for item in &batch.items {
        groups
            .entry(item.source_path.clone())
            .or_default()
            .push(item);
    }
    for (source_path, items) in groups {
        if !items.iter().all(|item| {
            item.status == ImportItemStatus::Done
                || (item.status == ImportItemStatus::Partial
                    && item.result.as_deref() == Some("archive_pending"))
        }) {
            continue;
        }
        let state = crate::modules::ingestion::adapters::sqlite::import_batch::get_mod_inbox_source_processing_state(
            pool,
            batch_id,
            &source_path,
        )
        .await?;
        if state
            .as_ref()
            .is_some_and(|state| state.source_processed_at.is_some())
        {
            continue;
        }
        let source = PathBuf::from(&source_path);
        let retains_source = items.iter().any(|item| item.staging_path.is_some());
        if !retains_source {
            crate::modules::ingestion::adapters::sqlite::import_batch::complete_mod_inbox_source_processing(
                pool,
                batch_id,
                &source_path,
                None,
            )
            .await?;
            continue;
        }
        let parent = source.parent().ok_or_else(|| {
            AppError::Validation("Mod Inbox source has no parent directory".to_string())
        })?;
        let processed = parent.join("Processed");
        std::fs::create_dir_all(&processed)?;
        let canonical_processed = processed.canonicalize()?;
        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::Validation("Mod Inbox source name is invalid".to_string()))?;
        let mut target = state
            .and_then(|state| state.processed_source_path)
            .map(PathBuf::from)
            .unwrap_or_else(|| collision_safe_file(&processed, file_name));
        validate_processed_target(&canonical_processed, &target)?;

        match (source.exists(), target.exists()) {
            (false, true) => {}
            (true, false) => {
                crate::modules::ingestion::adapters::sqlite::import_batch::plan_mod_inbox_source_processing(
                    pool,
                    batch_id,
                    &source_path,
                    &target.to_string_lossy(),
                )
                .await?;
                crate::platform::fs::file_utils::rename_cross_drive_fallback(&source, &target)?;
            }
            (true, true) => {
                target = collision_safe_file(&processed, file_name);
                crate::modules::ingestion::adapters::sqlite::import_batch::plan_mod_inbox_source_processing(
                    pool,
                    batch_id,
                    &source_path,
                    &target.to_string_lossy(),
                )
                .await?;
                crate::platform::fs::file_utils::rename_cross_drive_fallback(&source, &target)?;
            }
            (false, false) => {
                return Err(AppError::Validation(format!(
                    "Mod Inbox source and its planned Processed target are both unavailable: {}",
                    source.display()
                )));
            }
        }
        crate::modules::ingestion::adapters::sqlite::import_batch::complete_mod_inbox_source_processing(
            pool,
            batch_id,
            &source_path,
            Some(&target.to_string_lossy()),
        )
        .await?;
    }
    Ok(())
}

fn validate_processed_target(processed_root: &Path, target: &Path) -> Result<(), AppError> {
    let parent = target.parent().ok_or_else(|| {
        AppError::Validation("Processed target has no parent directory".to_string())
    })?;
    let canonical_parent = parent.canonicalize()?;
    if canonical_parent == processed_root {
        Ok(())
    } else {
        Err(AppError::Security(format!(
            "Planned Processed target escapes the Mod Inbox: {}",
            target.display()
        )))
    }
}

async fn finalize_batch_after_metadata(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
) -> Result<bool, AppError> {
    if batch.flow == ImportFlow::ReadyToMove {
        if let Err(error) = finalize_ready_to_move_archives(pool, &batch.id).await {
            crate::modules::ingestion::adapters::sqlite::import_batch::mark_ready_to_move_archive_pending(
                pool,
                &batch.id,
                &error.to_string(),
            )
            .await?;
            return Ok(true);
        }
        crate::modules::ingestion::adapters::sqlite::import_batch::complete_ready_to_move_archive_pending(pool, &batch.id)
            .await?;
    }
    let final_status =
        crate::modules::ingestion::adapters::sqlite::import_batch::finish_batch_from_items(
            pool, &batch.id,
        )
        .await?;
    if final_status
        == crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Done
    {
        cleanup_import_staging(app, &batch.id)?;
    }
    Ok(false)
}

async fn resume_ready_to_move_archive_finalization(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    batch: &crate::modules::ingestion::application::import_batch::types::ImportBatch,
) -> Result<ImportBatchReport, AppError> {
    let archive_failed = finalize_batch_after_metadata(app, pool, batch).await?;
    Ok(recovery_report(&batch.id, 0, u32::from(archive_failed)))
}

fn collision_safe_file(parent: &Path, file_name: &str) -> PathBuf {
    let direct = parent.join(file_name);
    if !direct.exists() {
        return direct;
    }
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("archive");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 2..=999 {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem} ({}).zip", uuid::Uuid::new_v4()))
}

fn report_for(
    batch_id: &str,
    plans: &[PlannedMove],
    collisions: &BTreeSet<String>,
    partial: bool,
) -> ImportBatchReport {
    ImportBatchReport {
        batch_id: batch_id.to_string(),
        moved: plans.len().saturating_sub(collisions.len()) as u32,
        reallocated: plans
            .iter()
            .filter(|plan| plan.item.decision == ImportDecision::Reallocate)
            .count() as u32,
        created_canonical_folders: plans
            .iter()
            .filter(|plan| plan.creates_object && !collisions.contains(&plan.item.id))
            .count() as u32,
        skipped: collisions.len() as u32,
        collisions: collisions.len() as u32,
        metadata_pending: 0,
        failed: u32::from(partial),
    }
}

fn result_label<T, E: std::fmt::Display>(result: Result<T, E>) -> String {
    match result {
        Ok(_) => "ok".to_string(),
        Err(error) => error.to_string(),
    }
}
