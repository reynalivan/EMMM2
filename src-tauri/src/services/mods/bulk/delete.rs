//! Bulk soft-delete (move to trash) across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::domain::collection::CollectionReferenceImpact;
use crate::repo::mod_repo;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::mods::trash;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

pub async fn bulk_delete(
    app: &AppHandle,
    config: &crate::services::config::ConfigService,
    pool: &SqlitePool,
    state: &WatcherState,
    paths: Vec<String>,
    game_id: Option<String>,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        crate::domain::errors::AppError::Io(format!("Failed to get app data dir: {}", e))
    })?;
    let trash_dir = crate::services::mods::trash::trash_dir_under(&app_data_dir);

    // One guard across the whole batch: no watcher-event leaks between items.
    let _suppression = SuppressionGuard::new(&state.suppressor);

    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir).map_err(|e| {
            crate::domain::errors::AppError::Io(format!("Failed to create trash dir: {}", e))
        })?;
    }

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
    let mut db_deletes = Vec::new();
    let mut collection_impact = CollectionReferenceImpact::default();

    // Opt-O: Batch progress — emit every N items
    let progress_interval = std::cmp::max(1, total / 10);

    for (i, path) in paths.iter().enumerate() {
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

        match trash::move_to_trash_guarded(state, &trash_dir, path.clone(), game_id.clone()).await {
            Ok(_) => {
                db_deletes.push(path.clone());
                success.push(path.clone());
            }
            Err(e) => failures.push(BulkActionError {
                path: path.clone(),
                error: e,
            }),
        }
    }

    if !db_deletes.is_empty() {
        // Detect which corridors were affected BEFORE deleting from DB or after if we still have the paths
        // Report the removed mods to any collection that referenced them.
        if let Some(gid) = &game_id {
            // Get mod path to compute relative paths
            let mp = crate::repo::game_repo::get_mod_path(pool, gid)
                .await
                .ok()
                .flatten();
            if let Some(base_path) = mp {
                let base = Path::new(&base_path);
                let relatives: Vec<String> = db_deletes
                    .iter()
                    .map(|p| {
                        Path::new(p)
                            .strip_prefix(base)
                            .map(|sp| sp.to_string_lossy().to_string())
                            .unwrap_or_else(|_| p.clone())
                    })
                    .collect();
                for relative in &relatives {
                    let impact =
                        crate::services::collection_service::handle_mod_missing(pool, relative)
                            .await
                            .unwrap_or_default();
                    collection_impact.merge(impact);
                }
            }
        }

        if let Err(e) = mod_repo::batch_delete_by_path(pool, &db_deletes).await {
            log::error!("Failed batch deleting mod paths from DB: {}", e);
        }

        // Trigger dirty state for cada affected corridor
        if let Some(gid) = &game_id {
            crate::services::app::runtime_effects::finalize_mutation(
                pool,
                config,
                gid,
                crate::services::app::runtime_effects::MutationOutcome::full_game(),
            )
            .await;
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

    // Convergence: scoped disk reconcile guarantees DB matches disk even if a
    // manual sync step above missed a case.
    if !success.is_empty() {
        if let Some(gid) = &game_id {
            if let Err(error) = emit_internal_disk_reconcile(app, pool, gid, success.clone()).await
            {
                log::warn!("Post-bulk-delete disk reconcile failed: {error}");
            }
        }
    }

    Ok(BulkResult::with_collection_impact(
        success,
        failures,
        collection_impact,
        Vec::new(),
    ))
}
