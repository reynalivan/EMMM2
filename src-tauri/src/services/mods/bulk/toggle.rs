//! Bulk enable/disable across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::workspace::WorkspacePathRewrite;
use crate::repo::mod_repo;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::mods::core_ops::toggle_mod_inner;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use sqlx::SqlitePool;
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// Bulk toggle mods on disk and sync DB.
///
/// `mods_path` must be provided and already validated by the caller (command layer).
/// Paths in `paths` are absolute; DB updates use relative paths computed from `mods_path`.
#[allow(clippy::too_many_arguments)] // Bulk service needs app/config/pool/watcher context plus explicit user selection.
pub async fn bulk_toggle(
    app: &AppHandle,
    config: &crate::services::config::ConfigService,
    pool: &SqlitePool,
    state: &WatcherState,
    mods_path: &str,
    game_id: &str,
    paths: Vec<String>,
    enable: bool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    // One guard across the whole batch: no watcher-event leaks between items.
    let _suppression = SuppressionGuard::new(&state.suppressor);

    let total = paths.len();
    let action_label = if enable { "Enabling" } else { "Disabling" };
    let new_status_enum = if enable {
        crate::domain::models::ItemStatus::Enabled
    } else {
        crate::domain::models::ItemStatus::Disabled
    };

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
    // (old_abs, new_abs, ItemStatus) — for DB batch update
    let mut db_updates = Vec::new();

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
                // Convert absolute paths to relative for DB storage
                let old_rel = Path::new(path)
                    .strip_prefix(mods_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.clone());
                let new_rel = Path::new(&new_abs_path)
                    .strip_prefix(mods_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| new_abs_path.clone());

                db_updates.push((old_rel.clone(), new_rel.clone(), new_status_enum));
                success.push(new_abs_path.clone());

                if old_rel != new_rel {
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

    if !db_updates.is_empty() {
        if let Err(e) = mod_repo::batch_update_path_and_status(pool, &db_updates).await {
            log::error!("Failed batch updating mod paths after bulk toggle: {}", e);
        }

        let _ = crate::repo::runtime_projection_repo::rebuild_game_projection(pool, game_id).await;
    }

    // Trigger Dirty State: Register unsaved changes for the affected corridors
    if !db_updates.is_empty() {
        // Collect subset of relative paths to check which corridors are affected
        let rel_paths: Vec<String> = db_updates
            .iter()
            .map(|(_, new_rel, _)| new_rel.clone())
            .collect();
        let safe_contexts = mod_repo::get_distinct_corridors_for_folders(pool, game_id, &rel_paths)
            .await
            .unwrap_or_default();
        let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
            pool,
            config,
            state.suppressor.clone(),
            game_id,
            &safe_contexts,
            true,
            true,
        )
        .await;
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

    // Convergence: scoped disk reconcile guarantees DB matches disk even if a
    // manual sync step above missed a case.
    if !success.is_empty() {
        if let Err(error) = emit_internal_disk_reconcile(app, pool, game_id, success.clone()).await
        {
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
