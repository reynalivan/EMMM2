//! Bulk enable/disable across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::workspace::WorkspacePathRewrite;
use crate::services::disk_reconcile::emit::run_internal_disk_reconcile;
use crate::services::mods::core_ops::toggle_mod_inner;
use crate::services::scanner::watcher::WatcherState;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

/// Bulk toggle mods on disk. The DB converges via the trailing scoped
/// reconcile — the single writer of status/path columns.
///
/// Paths in `paths` are absolute and already validated by the command layer.
pub async fn bulk_toggle(
    app: &AppHandle,
    pool: &SqlitePool,
    state: &WatcherState,
    game_id: &str,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    // One path-scoped guard across the whole batch: toggle renames keep
    // identity, so each selected path covers both its spellings.
    let _suppression = state.suppressor.suppress_paths(paths.iter());

    let total = paths.len();
    let action_label = if enable { "Enabling" } else { "Disabling" };

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("{} {} mods...", action_label, total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let collection_impact = CollectionReferenceImpact::default();
    let mut path_rewrites = Vec::new();

    // Opt-O: Batch progress — emit every N items to reduce IPC overhead
    let progress_interval = std::cmp::max(1, total / 10);

    for (i, path) in paths.iter().enumerate() {
        if i % progress_interval == 0 || i == total - 1 {
            let _ = app.emit(
                "bulk-progress",
                BulkProgressPayload {
                    label: format!("{} {}/{}", action_label, i + 1, total),
                    current: i + 1,
                    total,
                    active: true,
                },
            );
        }

        match toggle_mod_inner(state, path.clone(), enable).await {
            Ok(new_abs_path) => {
                success.push(new_abs_path.clone());

                if new_abs_path != *path {
                    path_rewrites.push(WorkspacePathRewrite {
                        old_path: path.clone(),
                        new_path: new_abs_path,
                    });
                }
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: "Done".to_string(),
            current: total,
            total,
            active: false,
        },
    );

    // Single writer: the scoped reconcile is what writes the rows. Quiet (no
    // frontend event) — the bulk mutation's caller publishes its own refresh,
    // and the event would trigger a second invalidation round.
    if !success.is_empty() {
        if let Err(error) = run_internal_disk_reconcile(app, pool, game_id, success.clone()).await {
            log::warn!("Post-bulk-toggle disk reconcile failed: {error}");
        }
    }

    Ok(BulkResult::with_collection_impact(
        success,
        failures,
        collection_impact,
        path_rewrites,
    ))
}
