//! Bulk soft-delete (move to trash) across many mod folders.

use super::types::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::domain::collection::CollectionReferenceImpact;
use crate::repo::mod_repo;
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::mods::trash;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

#[allow(clippy::too_many_arguments)] // Mirrors the command boundary's argument list.
pub async fn bulk_delete(
    app: &AppHandle,
    config: &crate::services::config::ConfigService,
    pool: &SqlitePool,
    state: &WatcherState,
    paths: Vec<String>,
    game_id: &str,
    cancel: &AtomicBool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        crate::domain::errors::AppError::Io(format!("Failed to get app data dir: {}", e))
    })?;
    let trash_dir = crate::services::mods::trash::trash_dir_under(&app_data_dir);

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
    let mut db_deletes = Vec::new();
    let mut collection_impact = CollectionReferenceImpact::default();

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

        match trash::move_to_trash_guarded(
            state,
            &trash_dir,
            path.clone(),
            Some(game_id.to_string()),
        )
        .await
        {
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
        collection_impact = collect_collection_impact(pool, game_id, &db_deletes).await;

        if let Err(e) = mod_repo::batch_delete_by_path(pool, game_id, &db_deletes).await {
            log::error!("Failed batch deleting mod paths from DB: {}", e);
        }

        crate::services::app::runtime_effects::finalize_mutation(
            pool,
            config,
            game_id,
            crate::services::app::runtime_effects::MutationOutcome::full_game(),
        )
        .await;
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

    // Convergence: scoped disk reconcile guarantees DB matches disk even if a
    // manual sync step above missed a case.
    if !success.is_empty() {
        if let Err(error) = emit_internal_disk_reconcile(app, pool, game_id, success.clone()).await
        {
            log::warn!("Post-bulk-delete disk reconcile failed: {error}");
        }
    }

    Ok(BulkResult::with_collection_impact(
        success,
        failures,
        collection_impact,
        Vec::new(),
    ))
}

async fn collect_collection_impact(
    pool: &SqlitePool,
    game_id: &str,
    deleted_paths: &[String],
) -> CollectionReferenceImpact {
    let Some(mods_path) = resolve_mods_path(pool, game_id).await else {
        return CollectionReferenceImpact::default();
    };
    let relative_paths = relative_deleted_paths(&mods_path, deleted_paths);
    scan_collection_impact(pool, &relative_paths).await
}

async fn resolve_mods_path(pool: &SqlitePool, game_id: &str) -> Option<String> {
    match crate::repo::game_repo::get_mod_path(pool, game_id).await {
        Ok(mods_path) => mods_path,
        Err(error) => {
            log::error!("Failed resolving mods path for collection impact scan: {error}");
            None
        }
    }
}

fn relative_deleted_paths(mods_path: &str, deleted_paths: &[String]) -> Vec<String> {
    let base = Path::new(&mods_path);
    deleted_paths
        .iter()
        .map(|path| {
            Path::new(path)
                .strip_prefix(base)
                .map(|relative| relative.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone())
        })
        .collect()
}

async fn scan_collection_impact(
    pool: &SqlitePool,
    relative_paths: &[String],
) -> CollectionReferenceImpact {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            log::error!("Failed opening collection impact transaction: {error}");
            return CollectionReferenceImpact::default();
        }
    };
    let collection_impact = scan_collection_impact_tx(&mut transaction, relative_paths).await;
    if let Err(error) = transaction.commit().await {
        log::error!("Failed committing collection impact scan: {error}");
    }
    collection_impact
}

async fn scan_collection_impact_tx(
    transaction: &mut sqlx::SqliteConnection,
    relative_paths: &[String],
) -> CollectionReferenceImpact {
    let mut combined_impact = CollectionReferenceImpact::default();
    for relative_path in relative_paths {
        match crate::services::collection_service::handle_mod_missing_tx(
            &mut *transaction,
            relative_path,
        )
        .await
        {
            Ok(impact) => combined_impact.merge(impact),
            Err(error) => {
                log::error!(
                    "Failed scanning collection impact for deleted mod {relative_path}: {error}"
                );
            }
        }
    }
    combined_impact
}
