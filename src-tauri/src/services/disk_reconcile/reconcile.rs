//! Disk Reconcile keeps the runtime projection aligned with filesystem reality.
//! Do not add MasterDB matching logic here.

use std::collections::BTreeSet;
use std::path::Path;

use crate::domain::collection::CollectionReferenceImpact;
use crate::repo::object_repo::ObjectRuntimeDescriptor;
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::disk_snapshot::collect_disk_projection;
use crate::services::disk_reconcile::helpers::normalize_runtime_name;
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

fn diff_cleared_paths(
    before: &[ObjectRuntimeDescriptor],
    after: &[ObjectRuntimeDescriptor],
) -> Vec<String> {
    let before_roots = runtime_roots(before);
    let after_roots = runtime_roots(after);

    before_roots.difference(&after_roots).cloned().collect()
}

fn merge_changed_roots(
    changed_roots: &[String],
    before: &[ObjectRuntimeDescriptor],
    after: &[ObjectRuntimeDescriptor],
) -> Vec<String> {
    let mut roots: BTreeSet<String> = changed_roots.iter().cloned().collect();
    let before_roots = runtime_roots(before);
    let after_roots = runtime_roots(after);
    roots.extend(before_roots.symmetric_difference(&after_roots).cloned());
    roots.into_iter().collect()
}

fn collect_runtime_file_changed(changed_paths: &[String]) -> bool {
    changed_paths
        .iter()
        .any(|value| is_runtime_relevant_file(Path::new(value)))
}

fn is_source_unavailable_error(error: &str) -> bool {
    error.starts_with("Disk Reconcile mods path is unavailable:")
        || error.starts_with("Failed to read directory")
        || error.starts_with("Failed to read directory entry")
        || error.starts_with("Failed to read file type")
}

fn runtime_mod_sample_name(mods_path: &Path, changed_path: &str) -> Option<String> {
    let relative = Path::new(changed_path).strip_prefix(mods_path).ok()?;
    let mut components = relative.components();
    components.next()?;
    let mod_component = components.next()?;
    Some(normalize_runtime_name(
        &mod_component.as_os_str().to_string_lossy(),
    ))
}

fn record_runtime_modifications(
    mods_path: &Path,
    changed_paths: &[String],
    change_summary: &mut ChangeSummaryBuilder,
) {
    let mut seen = BTreeSet::new();

    for changed_path in changed_paths {
        if !is_runtime_relevant_file(Path::new(changed_path)) {
            continue;
        }

        let relative = match Path::new(changed_path).strip_prefix(mods_path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let dedupe_key = relative
            .components()
            .take(2)
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        if dedupe_key.len() != 2 || !seen.insert(dedupe_key.join("/")) {
            continue;
        }

        if let Some(sample_name) = runtime_mod_sample_name(mods_path, changed_path) {
            change_summary.record_mod_modified(&sample_name);
        }
    }
}

/// Disk Reconcile updates the runtime projection from filesystem reality only.
/// Runtime-discovered folders remain `Other` until the explicit Deep Match Scanner runs.
pub async fn reconcile_disk_projection(
    request: ReconcileDiskProjectionRequest<'_>,
) -> Result<ReconcileOutcome, String> {
    let pool = request.pool;
    let game_id = request.game_id;
    let mods_path = request.mods_path;
    let safe_mode_keywords = request.safe_mode_keywords;
    let reason = request.reason;
    let changed_paths = request.changed_paths;
    let force_full = request.force_full;
    let watcher_events = request.watcher_events;

    let mut changed_roots = collect_changed_roots(mods_path, changed_paths);
    let thumbnail_roots = collect_thumbnail_roots(mods_path, changed_paths);
    let runtime_file_changed = collect_runtime_file_changed(changed_paths);
    if !mods_path.exists() || !mods_path.is_dir() {
        return Ok(ReconcileOutcome {
            status: DiskReconcileStatus::SourceUnavailable,
            error_message: Some(format!(
                "Disk Reconcile mods path is unavailable: {}",
                mods_path.display()
            )),
            changed_roots,
            thumbnail_roots,
            objects_changed: false,
            folders_changed: false,
            runtime_file_changed,
            cleared_selection_paths: Vec::new(),
            path_updates: Vec::new(),
            collection_reference_impact: CollectionReferenceImpact::default(),
            change_summary: ChangeSummaryBuilder::default().build(),
        });
    }

    let should_reconcile = force_full
        || !matches!(reason, DiskReconcileReason::WatcherBatch)
        || !changed_roots.is_empty();

    let before_descriptors = crate::repo::object_repo::get_runtime_descriptors(pool, game_id)
        .await
        .map_err(|error| error.to_string())?;

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
            Err(error) if is_source_unavailable_error(&error) => {
                return Ok(ReconcileOutcome {
                    status: DiskReconcileStatus::SourceUnavailable,
                    error_message: Some(error),
                    changed_roots,
                    thumbnail_roots,
                    objects_changed: false,
                    folders_changed: false,
                    runtime_file_changed,
                    cleared_selection_paths: Vec::new(),
                    path_updates: Vec::new(),
                    collection_reference_impact: CollectionReferenceImpact::default(),
                    change_summary: change_summary.build(),
                });
            }
            Err(error) => return Err(error),
        };
        let mut tx = pool.begin().await.map_err(|error| error.to_string())?;

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

        let (objects_changed_tx, folders_changed_tx) = reconcile_projection_in_tx(
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

        tx.commit().await.map_err(|error| error.to_string())?;
        crate::services::runtime_projection_service::rebuild_game_projection(pool, game_id)
            .await
            .map_err(|error| error.to_string())?;

        objects_changed = objects_changed_tx;
        folders_changed = folders_changed_tx;

        let after_descriptors = crate::repo::object_repo::get_runtime_descriptors(pool, game_id)
            .await
            .map_err(|error| error.to_string())?;
        cleared_selection_paths = diff_cleared_paths(&before_descriptors, &after_descriptors);

        if objects_changed || folders_changed {
            changed_roots =
                merge_changed_roots(&changed_roots, &before_descriptors, &after_descriptors);
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
