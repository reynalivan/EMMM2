use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_dir_in_configured_roots;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::archive::{extract_archive, ArchiveFormat, ExtractOptions};
use crate::services::mods::bulk::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

#[specta::specta]
#[tauri::command]
pub async fn import_mods_from_paths(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    config: State<'_, ConfigService>,
    paths: Vec<String>,
    target_dir: String,
) -> Result<BulkResult, AppError> {
    let _lock = op_lock.acquire().await?;
    validate_dir_in_configured_roots(&config, &target_dir)?;
    let total = paths.len();

    let _ = app.emit(
        "bulk-progress",
        BulkProgressPayload {
            label: format!("Importing {} items...", total),
            current: 0,
            total,
            active: true,
        },
    );

    let mut success = Vec::new();
    let mut failures = Vec::new();
    let target = Path::new(&target_dir);

    if !target.exists() || !target.is_dir() {
        return Err(AppError::NotFound(format!(
            "Target directory does not exist: {}",
            target_dir
        )));
    }

    for (i, path_str) in paths.iter().enumerate() {
        let _ = app.emit(
            "bulk-progress",
            BulkProgressPayload {
                label: format!("Importing {}/{}", i + 1, total),
                current: i + 1,
                total,
                active: true,
            },
        );

        let path = Path::new(&path_str);
        if !path.exists() {
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: AppError::Io("Source path does not exist".to_string()),
            });
            continue;
        }

        if ArchiveFormat::detect(path).is_some() {
            handle_archive_import(&state, path, target, path_str, &mut success, &mut failures);
            continue;
        }

        let file_name = match path.file_name() {
            Some(n) => n,
            None => {
                failures.push(BulkActionError {
                    path: path_str.clone(),
                    error: AppError::Io("Invalid file name".to_string()),
                });
                continue;
            }
        };

        let dest = target.join(file_name);
        if dest.exists() {
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: AppError::Io("Destination already exists".to_string()),
            });
            continue;
        }

        let _guard = SuppressionGuard::new(&state.suppressor);

        if let Err(e) =
            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(path, &dest)
        {
            log::warn!("Move failed (fallback failed): {}", e);
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: AppError::Io(format!("Failed to move: {}", e)),
            });
        } else {
            success.push(path_str.to_string());
        }
    }

    // Single-writer: watcher events were suppressed during the moves, so the
    // scoped reconcile is what writes the new rows.
    reconcile_import_target(&app, pool.inner(), &config, target).await;

    Ok(BulkResult::new(success, failures))
}

/// Reconcile the game whose mods root contains `target` after an import.
/// Failure is logged, not fatal — the files already landed on disk.
async fn reconcile_import_target(
    app: &AppHandle,
    pool: &sqlx::SqlitePool,
    config: &ConfigService,
    target: &Path,
) {
    let Some(game_id) = config.game_id_for_path(target) else {
        log::warn!(
            "Import target not under any configured mods root; skipping reconcile: {}",
            target.display()
        );
        return;
    };

    if let Err(error) = crate::services::disk_reconcile::emit::emit_internal_disk_reconcile(
        app,
        pool,
        &game_id,
        vec![target.to_string_lossy().to_string()],
    )
    .await
    {
        log::warn!("Post-import disk reconcile failed: {error}");
    }
}

fn handle_archive_import(
    state: &WatcherState,
    path: &Path,
    target: &Path,
    path_str: &str,
    success: &mut Vec<String>,
    failures: &mut Vec<BulkActionError>,
) {
    let _guard = SuppressionGuard::new(&state.suppressor);

    match extract_archive(path, target, ExtractOptions::default()) {
        Ok(result) => {
            if !result.success {
                failures.push(BulkActionError {
                    path: path_str.to_string(),
                    error: AppError::Io(
                        result
                            .error
                            .unwrap_or_else(|| "Unknown extraction error".into()),
                    ),
                });
                return;
            }

            for extracted_dest in &result.dest_paths {
                let extracted_path = Path::new(extracted_dest);
                match crate::services::mods::arrival::land_disabled(extracted_path, target) {
                    Ok(final_path) => success.push(final_path.to_string_lossy().to_string()),
                    Err(error) => failures.push(BulkActionError {
                        path: extracted_dest.clone(),
                        error,
                    }),
                }
            }
        }
        Err(e) => failures.push(BulkActionError {
            path: path_str.to_string(),
            error: e,
        }),
    }
}

#[specta::specta]
#[tauri::command]
pub async fn ingest_dropped_folders(
    app: AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    config: State<'_, ConfigService>,
    paths: Vec<String>,
    mods_path: String,
) -> Result<Vec<String>, AppError> {
    let _lock = op_lock.acquire().await?;
    validate_dir_in_configured_roots(&config, &mods_path)?;
    let moved = ingest_dropped_folders_inner(&state, paths, mods_path.clone()).await?;

    if !moved.is_empty() {
        reconcile_import_target(&app, pool.inner(), &config, Path::new(&mods_path)).await;
    }

    Ok(moved)
}

/// Moves dropped folders into the mods root; returns the moved folder names.
pub async fn ingest_dropped_folders_inner(
    state: &WatcherState,
    paths: Vec<String>,
    mods_path: String,
) -> Result<Vec<String>, AppError> {
    let target = Path::new(&mods_path);

    if !target.exists() || !target.is_dir() {
        return Err(AppError::NotFound(format!(
            "Mods path does not exist: {mods_path}"
        )));
    }

    let mut moved = Vec::new();

    let _guard = SuppressionGuard::new(&state.suppressor);

    for src_str in &paths {
        let src = Path::new(src_str);
        if !src.is_dir() {
            continue;
        }

        let Some(basename) = src.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };

        let dest = target.join(&basename);
        if dest.exists() {
            continue;
        }

        if crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, &dest).is_ok() {
            moved.push(basename);
        }
    }

    Ok(moved)
}

#[cfg(test)]
#[path = "tests/mod_import_cmds_tests.rs"]
mod tests;
