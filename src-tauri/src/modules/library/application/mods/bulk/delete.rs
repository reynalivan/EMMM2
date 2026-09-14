//! Bulk soft-delete (move to trash) across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::library::application::mods::trash;
use crate::modules::library::application::mods::trash::PreparedTrashMove;
use crate::modules::workspace::application::scanner::watcher::{SuppressionGuard, WatcherState};
use crate::platform::fs::guard::ValidatedPath;
use crate::shared::errors::AppError;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub struct PreparedBulkDelete {
    items: Vec<PreparedTrashMove>,
}

impl PreparedBulkDelete {
    pub fn journal_steps(&self) -> Vec<(u32, std::path::PathBuf, std::path::PathBuf)> {
        self.items
            .iter()
            .enumerate()
            .map(|(sequence, item)| {
                (
                    sequence as u32,
                    item.source().to_path_buf(),
                    item.quarantine().to_path_buf(),
                )
            })
            .collect()
    }
}

pub struct BulkDeleteExecution {
    pub result: BulkResult,
    pub applied_sequences: Vec<u32>,
}

pub fn prepare_bulk_delete(paths: &[ValidatedPath]) -> Result<PreparedBulkDelete, AppError> {
    Ok(PreparedBulkDelete {
        items: paths
            .iter()
            .map(|path| trash::prepare_trash_move(path.as_ref()))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn execute_prepared_bulk_delete(
    app: &AppHandle,
    state: &WatcherState,
    prepared: &PreparedBulkDelete,
    cancel: &AtomicBool,
) -> BulkDeleteExecution {
    let total = prepared.items.len();
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: "common:bulk_progress.deleting".to_string(),
            current: 0,
            total,
            active: true,
        },
    );
    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut applied_sequences = Vec::new();
    let progress_interval = std::cmp::max(1, total / 10);
    let mut cancelled = false;
    let mut processed_count = 0;
    for (index, item) in prepared.items.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        if index % progress_interval == 0 || index == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: "common:bulk_progress.deleting".to_string(),
                    current: index + 1,
                    total,
                    active: true,
                },
            );
        }
        match item.execute(state) {
            Ok(()) => {
                success.push(item.source().to_string_lossy().into_owned());
                applied_sequences.push(index as u32);
            }
            Err(error) => failures.push(BulkActionError {
                path: item.source().to_string_lossy().into_owned(),
                error,
            }),
        }
        processed_count += 1;
    }
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: if cancelled {
                "common:bulk_progress.cancelled"
            } else {
                "common:bulk_progress.done"
            }
            .to_string(),
            current: processed_count,
            total,
            active: false,
        },
    );
    BulkDeleteExecution {
        result: BulkResult::with_collection_impact(
            success,
            failures,
            CollectionReferenceImpact::default(),
            Vec::new(),
        )
        .with_execution_state(cancelled, processed_count, total),
        applied_sequences,
    }
}

pub fn rollback_prepared_bulk_delete(
    state: &WatcherState,
    prepared: &PreparedBulkDelete,
    applied_sequences: &[u32],
) -> Result<(), AppError> {
    let applied = applied_sequences.iter().copied().collect::<HashSet<_>>();
    let mut failures = Vec::new();
    for (sequence, item) in prepared.items.iter().enumerate().rev() {
        if applied.contains(&(sequence as u32)) {
            if let Err(error) = item.rollback(state) {
                failures.push(error.to_string());
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Bulk trash rollback was incomplete: {}",
            failures.join("; ")
        )))
    }
}

pub fn finalize_prepared_bulk_delete(
    prepared: &PreparedBulkDelete,
    applied_sequences: &[u32],
) -> Result<(), AppError> {
    let applied = applied_sequences.iter().copied().collect::<HashSet<_>>();
    let mut failures = Vec::new();
    for (sequence, item) in prepared.items.iter().enumerate() {
        if applied.contains(&(sequence as u32)) {
            if let Err(error) = item.finalize() {
                failures.push(error.to_string());
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Recycle Bin cleanup is pending: {}",
            failures.join("; ")
        )))
    }
}

#[allow(clippy::too_many_arguments)] // Mirrors the command boundary's argument list.
pub async fn bulk_delete(
    app: &AppHandle,
    state: &WatcherState,
    paths: Vec<String>,
    cancel: &AtomicBool,
) -> Result<BulkResult, crate::shared::errors::AppError> {
    // One guard across the whole batch: no watcher-event leaks between items.
    let _suppression = SuppressionGuard::new(&state.suppressor);

    let total = paths.len();
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: "common:bulk_progress.deleting".to_string(),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();

    // Opt-O: Batch progress — emit every N items
    let progress_interval = std::cmp::max(1, total / 10);

    let mut cancelled = false;
    let mut processed_count = 0;
    for (i, path) in paths.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }

        if i % progress_interval == 0 || i == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: "common:bulk_progress.deleting".to_string(),
                    current: i + 1,
                    total,
                    active: true,
                },
            );
        }

        match trash::move_to_trash_guarded(state, path.clone()).await {
            Ok(_) => success.push(path.clone()),
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
        processed_count += 1;
    }

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: if cancelled {
                "common:bulk_progress.cancelled"
            } else {
                "common:bulk_progress.done"
            }
            .to_string(),
            current: processed_count,
            total,
            active: false,
        },
    );

    Ok(BulkResult::with_collection_impact(
        success,
        failures,
        CollectionReferenceImpact::default(),
        Vec::new(),
    )
    .with_execution_state(cancelled, processed_count, total))
}
