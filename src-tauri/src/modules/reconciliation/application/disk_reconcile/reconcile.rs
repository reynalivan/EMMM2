//! Disk Reconcile keeps the runtime projection aligned with filesystem reality.
//! Do not add MasterDB matching logic here.

use crate::shared::errors::AppError;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use crate::modules::workspace::domain::normalizer::normalize_display_name;
use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::catalog::domain::objects::ObjectRuntimeDescriptor;
use crate::modules::reconciliation::application::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
    collect_scoped_disk_discovery_with_progress, DiskProjectionError, DiskSnapshotProgress,
};
use crate::modules::reconciliation::application::disk_reconcile::identity_conflicts::detect_folder_name_conflicts_from_census;
use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcilePathHint;
use crate::modules::reconciliation::application::disk_reconcile::path_classifier::{
    collect_changed_roots, collect_thumbnail_roots, is_runtime_relevant_file,
};
use crate::modules::reconciliation::application::disk_reconcile::projection_writer::{
    reconcile_projection_in_tx, ProjectionWriteRequest,
};
use crate::modules::reconciliation::application::disk_reconcile::rename_confirmation::detect_rename_confirmations;
use crate::modules::reconciliation::application::disk_reconcile::rename_healer::{
    apply_watcher_rename_hints, WatcherRenameHintsApplyRequest,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileChangeSummary, DiskReconcilePathUpdate, DiskReconcileReason, DiskReconcileStatus,
    FolderNameConflictGroup,
};
use crate::modules::workspace::application::scanner::watcher::ModWatchEvent;

#[derive(Debug, Clone)]
pub struct ReconcileOutcome {
    pub status: DiskReconcileStatus,
    pub error_message: Option<String>,
    pub folder_conflicts: Vec<FolderNameConflictGroup>,
    pub rename_confirmations: Vec<crate::modules::reconciliation::application::disk_reconcile::types::RenameConfirmationGroup>,
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
    pub path_hints: &'a [DiskReconcilePathHint],
    pub progress_reporter:
        Option<Arc<crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileProgressReporter>>,
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

fn has_mods_root_event(mods_path: &Path, changed_paths: &[String]) -> bool {
    changed_paths.iter().any(|changed_path| {
        let path = Path::new(changed_path);
        path == mods_path
            || path
                .to_string_lossy()
                .eq_ignore_ascii_case(&mods_path.to_string_lossy())
    })
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
        folder_conflicts: Vec::new(),
        rename_confirmations: Vec::new(),
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

fn internal_mutation_rename_evidence(
    mods_path: &Path,
    changed_paths: &[String],
) -> Vec<ModWatchEvent> {
    let mut paths_by_identity = std::collections::BTreeMap::<String, Vec<&str>>::new();
    for changed_path in changed_paths {
        let path = Path::new(changed_path);
        let Ok(relative) = path.strip_prefix(mods_path) else {
            continue;
        };
        let key = crate::shared::path_key::folder_path_key(&relative.to_string_lossy(), None);
        let paths = paths_by_identity.entry(key).or_default();
        if !paths
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(changed_path))
        {
            paths.push(changed_path);
        }
    }

    paths_by_identity
        .into_values()
        .filter_map(|paths| {
            if paths.len() != 2 {
                return None;
            }
            let existing = paths
                .iter()
                .filter(|path| Path::new(path).exists())
                .copied()
                .collect::<Vec<_>>();
            let missing = paths
                .iter()
                .filter(|path| !Path::new(path).exists())
                .copied()
                .collect::<Vec<_>>();
            if existing.len() != 1 || missing.len() != 1 {
                return None;
            }
            Some(ModWatchEvent::Renamed {
                from: missing[0].to_string(),
                to: existing[0].to_string(),
            })
        })
        .collect()
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
        path_hints,
        progress_reporter,
    } = request;

    let mut changed_roots = collect_changed_roots(mods_path, changed_paths);
    let mut thumbnail_roots = collect_thumbnail_roots(mods_path, changed_paths);
    for changed_path in changed_paths {
        let path = Path::new(changed_path);
        if crate::modules::reconciliation::application::disk_reconcile::path_classifier::is_thumbnail_path(path) {
            crate::platform::images::thumbnail_cache::ThumbnailCache::invalidate(path);
        }
    }
    let runtime_file_changed = collect_runtime_file_changed(changed_paths);
    let mut effective_watcher_events = watcher_events.unwrap_or_default().to_vec();
    effective_watcher_events.extend(path_hints.iter().map(|hint| ModWatchEvent::Renamed {
        from: mods_path.join(&hint.old_path).to_string_lossy().to_string(),
        to: mods_path.join(&hint.new_path).to_string_lossy().to_string(),
    }));
    if matches!(reason, DiskReconcileReason::InternalMutation) {
        effective_watcher_events
            .extend(internal_mutation_rename_evidence(mods_path, changed_paths));
    }
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

    let thumbnail_only_watcher_batch = matches!(reason, DiskReconcileReason::WatcherBatch)
        && !changed_paths.is_empty()
        && changed_paths.iter().all(|path| {
            crate::modules::reconciliation::application::disk_reconcile::path_classifier::is_thumbnail_path(Path::new(path))
        });
    if thumbnail_only_watcher_batch {
        return Ok(ReconcileOutcome {
            status: DiskReconcileStatus::Applied,
            error_message: None,
            folder_conflicts: Vec::new(),
            rename_confirmations: Vec::new(),
            changed_roots,
            thumbnail_roots,
            objects_changed: false,
            folders_changed: false,
            runtime_file_changed: false,
            cleared_selection_paths: Vec::new(),
            path_updates: Vec::new(),
            collection_reference_impact: CollectionReferenceImpact::default(),
            change_summary: DiskReconcileChangeSummary::default(),
        });
    }

    let mods_root_event = has_mods_root_event(mods_path, changed_paths);
    let should_reconcile = force_full
        || mods_root_event
        || !matches!(reason, DiskReconcileReason::WatcherBatch)
        || !changed_roots.is_empty();

    let before_descriptors =
        crate::modules::catalog::adapters::outbound::sqlite::object::get_runtime_descriptors(pool, game_id).await?;

    let mut objects_changed = false;
    let mut folders_changed = false;
    let mut cleared_selection_paths = Vec::new();
    let mut path_updates = Vec::new();
    let mut collection_reference_impact = CollectionReferenceImpact::default();
    let mut change_summary = ChangeSummaryBuilder::default();
    let mut folder_conflicts = Vec::new();
    let mut rename_confirmations = Vec::new();

    if runtime_file_changed {
        record_runtime_modifications(mods_path, changed_paths, &mut change_summary);
    }

    if should_reconcile {
        let requested_scoped = !force_full
            && !mods_root_event
            && should_run_scoped_disk_reconcile(reason, &changed_roots);
        // Discovery reads the global directory-name census, then strictly
        // classifies only changed roots unless that census proves a cross-root
        // identity ambiguity. All filesystem work remains on the blocking pool.
        let snapshot_path = mods_path.to_path_buf();
        let snapshot_roots = changed_roots.clone();
        let snapshot_progress = progress_reporter.clone();
        let snapshot = tokio::task::spawn_blocking(move || {
            let on_progress = |progress: DiskSnapshotProgress| {
                if let Some(reporter) = &snapshot_progress {
                    reporter.emit(
                        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePhase::ScanningRoots,
                        progress.completed_roots as u64,
                        Some(progress.total_roots as u64),
                        progress.current_root,
                    );
                }
            };
            collect_scoped_disk_discovery_with_progress(
                &snapshot_path,
                &snapshot_roots,
                requested_scoped,
                Some(&on_progress),
            )
        })
        .await?;
        let discovery = match snapshot {
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
        let scoped = discovery.scoped;
        let census = discovery.census;
        let projection = discovery.projection;
        if force_full {
            crate::platform::images::thumbnail_cache::ThumbnailCache::clear_memory();
            thumbnail_roots = projection
                .objects
                .iter()
                .map(|entry| entry.folder_path.clone())
                .collect();
        }
        if let Some(reporter) = &progress_reporter {
            reporter.emit(
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePhase::Projecting,
                0,
                None,
                None,
            );
        }

        folder_conflicts = detect_folder_name_conflicts_from_census(game_id, &census);
        let mut protected_object_keys = folder_conflicts
            .iter()
            .flat_map(|group| group.candidates.iter())
            .filter_map(|candidate| {
                projection
                    .objects
                    .iter()
                    .find(|entry| entry.absolute_path.to_string_lossy() == candidate.path)
                    .map(|entry| entry.folder_path_key.clone())
            })
            .collect::<std::collections::HashSet<_>>();
        let mut protected_mod_keys = folder_conflicts
            .iter()
            .flat_map(|group| group.candidates.iter())
            .filter_map(|candidate| {
                projection
                    .mods
                    .iter()
                    .find(|entry| entry.absolute_path.to_string_lossy() == candidate.path)
                    .map(|entry| entry.folder_path_key.clone())
            })
            .collect::<std::collections::HashSet<_>>();
        // Conflict groups are the complete read-only overlay for ambiguous
        // identities. Do not invent one writable DB representative: omitting
        // every candidate preserves a prior row through the protected-key prune
        // guards, while an identity with no prior row remains absent from the
        // runtime projection and collection signature until it is resolved.
        let projection = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskProjection {
            objects: projection
                .objects
                .iter()
                .filter(|entry| !protected_object_keys.contains(&entry.folder_path_key))
                .cloned()
                .collect(),
            mods: projection
                .mods
                .iter()
                .filter(|entry| {
                    !protected_object_keys.contains(&entry.object_folder_path_key)
                        && !protected_mod_keys.contains(&entry.folder_path_key)
                })
                .cloned()
                .collect(),
        };
        let rename_detection = detect_rename_confirmations(
            pool,
            game_id,
            mods_path,
            &projection,
            &effective_watcher_events,
        )
        .await?;
        protected_object_keys.extend(rename_detection.protected_object_keys);
        protected_mod_keys.extend(rename_detection.protected_mod_keys);
        rename_confirmations = rename_detection.groups;
        // Rename ambiguity protects only the candidate keys returned by the
        // scoped detector. The same writer transaction still converges every
        // unrelated object and mod in this snapshot.
        let projection = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskProjection {
            objects: projection
                .objects
                .into_iter()
                .filter(|entry| !protected_object_keys.contains(&entry.folder_path_key))
                .collect(),
            mods: projection
                .mods
                .into_iter()
                .filter(|entry| {
                    !protected_object_keys.contains(&entry.object_folder_path_key)
                        && !protected_mod_keys.contains(&entry.folder_path_key)
                })
                .collect(),
        };
        // Keep the write input coherent with the full preflight snapshot.
        // Pruning remains scoped below, but identity transitions (including
        // cross-root swaps) must be able to see every physical folder.
        let projection = &projection;
        let mut tx = pool.begin().await?;

        if !effective_watcher_events.is_empty() {
            apply_watcher_rename_hints(WatcherRenameHintsApplyRequest {
                conn: &mut tx,
                game_id,
                mods_path,
                safe_mode_keywords,
                watcher_events: &effective_watcher_events,
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
                projection,
                changed_roots: &changed_roots,
                force_full,
                path_updates: &mut path_updates,
                collection_reference_impact: &mut collection_reference_impact,
                change_summary: &mut change_summary,
                protected_object_keys: &protected_object_keys,
                protected_mod_keys: &protected_mod_keys,
            },
        )
        .await?;

        if scoped {
            crate::modules::workspace::adapters::outbound::sqlite::runtime_projection::refresh_projection_for_object_ids_tx(
                &mut tx,
                game_id,
                write_outcome.touched_object_ids.iter().cloned(),
            )
            .await?;
        } else {
            crate::modules::workspace::adapters::outbound::sqlite::runtime_projection::rebuild_game_projection_tx(&mut tx, game_id)
                .await?;
        }
        tx.commit().await?;

        objects_changed = write_outcome.objects_changed;
        folders_changed = write_outcome.folders_changed;

        let after_descriptors =
            crate::modules::catalog::adapters::outbound::sqlite::object::get_runtime_descriptors(pool, game_id).await?;

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
        status: if !rename_confirmations.is_empty() {
            DiskReconcileStatus::NeedsRenameConfirmation
        } else if folder_conflicts.is_empty() {
            DiskReconcileStatus::Applied
        } else {
            DiskReconcileStatus::AppliedWithFolderConflicts
        },
        error_message: None,
        folder_conflicts,
        rename_confirmations,
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
