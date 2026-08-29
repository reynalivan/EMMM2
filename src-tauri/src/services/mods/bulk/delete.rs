//! Bulk soft-delete (move to trash) across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::domain::collection::CollectionReferenceImpact;
use crate::services::mods::trash;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

#[allow(clippy::too_many_arguments)] // Mirrors the command boundary's argument list.
pub async fn bulk_delete(
    app: &AppHandle,
    state: &WatcherState,
    paths: Vec<String>,
    cancel: &AtomicBool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    // One guard across the whole batch: no watcher-event leaks between items.
    let _suppression = SuppressionGuard::new(&state.suppressor);

    let total = paths.len();
    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("Deleting {} mods...", total),
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
    for (i, path) in paths.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }

        if i % progress_interval == 0 || i == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: format!("Deleting {}/{}", i + 1, total),
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
    }

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: if cancelled { "Cancelled" } else { "Done" }.to_string(),
            current: total,
            total,
            active: false,
        },
    );

    Ok(BulkResult::with_collection_impact(
        success,
        failures,
        CollectionReferenceImpact::default(),
        Vec::new(),
    ))
}
