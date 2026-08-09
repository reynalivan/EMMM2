//! Disk Reconcile keeps the runtime projection aligned with filesystem reality.
//! Do not add MasterDB matching logic here.

use crate::domain::errors::AppError;
use std::collections::BTreeSet;
use std::path::Path;

use crate::common::normalizer::normalize_display_name;
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::objects::ObjectRuntimeDescriptor;
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::disk_snapshot::{
    collect_disk_projection, DiskProjectionError,
};
use crate::services::disk_reconcile::path_classifier::{
    collect_changed_roots, collect_thumbnail_roots, is_runtime_relevant_file,
};
use crate::services::disk_reconcile::projection_writer::{
    reconcile_projection_in_tx, ProjectionWriteRequest,
};
use crate::services::disk_reconcile::rename_healer::{
    apply_watcher_rename_hints, WatcherRenameHintsApplyRequest,
};
use crate::services::disk_reconcile::types::{
    DiskReconcileChangeSummary, DiskReconcilePathUpdate, DiskReconcileReason, DiskReconcileStatus,
};
use crate::services::scanner::watcher::ModWatchEvent;

#[derive(Debug, Clone)]
pub struct ReconcileOutcome {
    pub status: DiskReconcileStatus,
    pub error_message: Option<String>,
    pub changed_roots: Vec<String>,
    pub thumbnail_roots: Vec<String>,
    pub objects_changed: bool,
    pub folders_changed: bool,
    pub runtime_file_changed: bool,
    pub cleared_selection_paths: Vec<String>,
    pub path_updates: Vec<DiskReconcilePathUpdate>,
    pub collection_reference_impact: CollectionReferenceImpact,
    pub change_summary: DiskReconcileChangeSummary,
}

pub struct ReconcileDiskProjectionRequest<'a> {
    pub pool: &'a sqlx::SqlitePool,
    pub game_id: &'a str,
    pub mods_path: &'a Path,
    pub safe_mode_keywords: &'a [String],
    pub reason: &'a DiskReconcileReason,
    pub changed_paths: &'a [String],
    pub force_full: bool,
    pub watcher_events: Option<&'a [ModWatchEvent]>,
}

fn should_run_scoped_disk_reconcile(
    reason: &DiskReconcileReason,
    changed_roots: &[String],
) -> bool {
    if changed_roots.is_empty() {
        return false;
    }

    matches!(
        reason,
        DiskReconcileReason::WatcherBatch | DiskReconcileReason::InternalMutation
    )
}

fn runtime_roots(descriptors: &[ObjectRuntimeDescriptor]) -> BTreeSet<String> {
    descriptors
        .iter()
        .map(|entry| entry.folder_path.clone())
        .collect()
}

fn merge_changed_roots(
    changed_roots: &[String],
    before_roots: &BTreeSet<String>,
    after_roots: &BTreeSet<String>,
) -> Vec<String> {
    let mut roots: BTreeSet<String> = changed_roots.iter().cloned().collect();
    roots.extend(before_roots.symmetric_difference(after_roots).cloned());
    roots.into_iter().collect()
}

/// The mods root disappeared or could not be read. Everything the caller
/// already computed from the change list still stands; nothing was written.
fn source_unavailable(
    error_message: String,
    changed_roots: Vec<String>,
    thumbnail_roots: Vec<String>,
    runtime_file_changed: bool,
    change_summary: DiskReconcileChangeSummary,
) -> ReconcileOutcome {
    ReconcileOutcome {
        status: DiskReconcileStatus::SourceUnavailable,
        error_message: Some(error_message),
        changed_roots,
        thumbnail_roots,
        objects_changed: false,
        folders_changed: false,
        runtime_file_changed,
        cleared_selection_paths: Vec::new(),
        path_updates: Vec::new(),
        collection_reference_impact: CollectionReferenceImpact::default(),
        change_summary,
    }
}

fn collect_runtime_file_changed(changed_paths: &[String]) -> bool {
    changed_paths
        .iter()
        .any(|value| is_runtime_relevant_file(Path::new(value)))
}

fn record_runtime_modifications(
    mods_path: &Path,
    changed_paths: &[String],
    change_summary: &mut ChangeSummaryBuilder,
) {
    let mut seen_parents = BTreeSet::new();

    for changed_path in changed_paths {
        if !is_runtime_relevant_file(Path::new(changed_path)) {
            continue;
        }

        let Ok(relative) = Path::new(changed_path).strip_prefix(mods_path) else {
            continue;
        };

        // The mod a runtime file belongs to is the folder containing it —
        // fixed-index components misname an ini sitting directly in an object
        // root (reports the file) or nested under a container (reports the
        // container). A file directly in the mods root has no parent folder.
        let Some(parent) = relative
            .parent()
            .filter(|value| !value.as_os_str().is_empty())
        else {
            continue;
        };
        if !seen_parents.insert(parent.to_path_buf()) {
            continue;
        }

        if let Some(folder_name) = parent.file_name() {
            change_summary
                .record_mod_modified(&normalize_display_name(&folder_name.to_string_lossy()));
        }
    }
}

/// Disk Reconcile updates the runtime projection from filesystem reality only.
/// Runtime-discovered folders remain `Other` until the explicit Deep Match Scanner runs.
pub async fn reconcile_disk_projection(
    request: ReconcileDiskProjectionRequest<'_>,
) -> Result<ReconcileOutcome, AppError> {
    let ReconcileDiskProjectionRequest {
        pool,
        game_id,
        mods_path,
        safe_mode_keywords,
        reason,
        changed_paths,
        force_full,
        watcher_events,
    } = request;

    let mut changed_roots = collect_changed_roots(mods_path, changed_paths);
    let thumbnail_roots = collect_thumbnail_roots(mods_path, changed_paths);
    let runtime_file_changed = collect_runtime_file_changed(changed_paths);
    if !mods_path.exists() || !mods_path.is_dir() {
        return Ok(source_unavailable(
            format!(
                "Disk Reconcile mods path is unavailable: {}",
                mods_path.display()
            ),
            changed_roots,
            thumbnail_roots,
            runtime_file_changed,
            ChangeSummaryBuilder::default().build(),
        ));
    }

    let should_reconcile = force_full
        || !matches!(reason, DiskReconcileReason::WatcherBatch)
        || !changed_roots.is_empty();

    let before_descriptors =
        crate::repo::object_repo::get_runtime_descriptors(pool, game_id).await?;

    let mut objects_changed = false;
    let mut folders_changed = false;
    let mut cleared_selection_paths = Vec::new();
    let mut path_updates = Vec::new();
    let mut collection_reference_impact = CollectionReferenceImpact::default();
    let mut change_summary = ChangeSummaryBuilder::default();

    if runtime_file_changed {
        record_runtime_modifications(mods_path, changed_paths, &mut change_summary);
    }

    if should_reconcile {
        let scoped = !force_full && should_run_scoped_disk_reconcile(reason, &changed_roots);
        let projection = match collect_disk_projection(mods_path, &changed_roots, scoped) {
            Ok(value) => value,
            Err(DiskProjectionError::SourceUnavailable(message)) => {
                return Ok(source_unavailable(
                    message,
                    changed_roots,
                    thumbnail_roots,
                    runtime_file_changed,
                    change_summary.build(),
                ));
            }
            Err(error) => return Err(AppError::Internal(error.into_message())),
        };
        let mut tx = pool.begin().await?;

        if let Some(events) = watcher_events {
            apply_watcher_rename_hints(WatcherRenameHintsApplyRequest {
                conn: &mut tx,
                game_id,
                mods_path,
                safe_mode_keywords,
                watcher_events: events,
                path_updates: &mut path_updates,
                collection_reference_impact: &mut collection_reference_impact,
                change_summary: &mut change_summary,
            })
            .await?;
        }

        let write_outcome = reconcile_projection_in_tx(
            &mut tx,
            ProjectionWriteRequest {
                game_id,
                mods_path,
                safe_mode_keywords,
                projection: &projection,
                changed_roots: &changed_roots,
                force_full,
                path_updates: &mut path_updates,
                collection_reference_impact: &mut collection_reference_impact,
                change_summary: &mut change_summary,
            },
        )
        .await?;

        tx.commit().await?;
        if scoped {
            let touched_ids: Vec<String> =
                write_outcome.touched_object_ids.iter().cloned().collect();
            crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
                pool,
                game_id,
                &touched_ids,
                false,
            )
            .await?;
        } else {
            crate::repo::runtime_projection_repo::rebuild_game_projection(pool, game_id).await?;
        }

        objects_changed = write_outcome.objects_changed;
        folders_changed = write_outcome.folders_changed;

        let after_descriptors =
            crate::repo::object_repo::get_runtime_descriptors(pool, game_id).await?;

        // Both the cleared-selection diff and the changed-root merge compare the
        // same two descriptor sets; build each set once.
        let before_roots = runtime_roots(&before_descriptors);
        let after_roots = runtime_roots(&after_descriptors);
        cleared_selection_paths = before_roots.difference(&after_roots).cloned().collect();

        if objects_changed || folders_changed {
            changed_roots = merge_changed_roots(&changed_roots, &before_roots, &after_roots);
        }
    }

    Ok(ReconcileOutcome {
        status: DiskReconcileStatus::Applied,
        error_message: None,
        changed_roots,
        thumbnail_roots,
        objects_changed,
        folders_changed,
        runtime_file_changed,
        cleared_selection_paths,
        path_updates,
        collection_reference_impact,
        change_summary: change_summary.build(),
    })
}
