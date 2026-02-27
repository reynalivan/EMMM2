use crate::commands::mods::mod_bulk_cmds::{BulkActionError, BulkProgressPayload, BulkResult};
use crate::services::core::operation_lock::OperationLock;
use crate::services::mod_files::archive::{extract_archive, ArchiveFormat};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use crate::DISABLED_PREFIX;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, serde::Deserialize)]
pub enum ImportStrategy {
    Raw,
    AutoOrganize,
}

#[tauri::command]
pub async fn import_mods_from_paths(
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    target_dir: String,
    strategy: ImportStrategy,
    db_json: Option<String>,
) -> Result<BulkResult, String> {
    let _lock = op_lock.acquire().await?;
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
        return Err(format!("Target directory does not exist: {}", target_dir));
    }

    let db = if let ImportStrategy::AutoOrganize = strategy {
        if let Some(json) = db_json {
            Some(crate::services::scanner::deep_matcher::MasterDb::from_json(
                &json,
            )?)
        } else {
            return Err("Auto-Organize requires db_json".to_string());
        }
    } else {
        None
    };

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
                error: "Source path does not exist".to_string(),
            });
            continue;
        }

        if let (ImportStrategy::AutoOrganize, Some(master_db)) = (&strategy, &db) {
            let _guard = SuppressionGuard::new(&state.suppressor);
            match crate::services::scanner::core::organizer::organize_mod(path, target, master_db) {
                Ok(res) => success.push(res.new_path.to_string_lossy().to_string()),
                Err(e) => failures.push(BulkActionError {
                    path: path_str.clone(),
                    error: e.to_string(),
                }),
            }
            continue;
        }

        if let Some(_) = ArchiveFormat::from_path(path) {
            handle_archive_import(
                &state,
                path,
                target,
                &db,
                path_str,
                &mut success,
                &mut failures,
            );
            continue;
        }

        let file_name = match path.file_name() {
            Some(n) => n,
            None => {
                failures.push(BulkActionError {
                    path: path_str.clone(),
                    error: "Invalid file name".to_string(),
                });
                continue;
            }
        };

        let dest = target.join(file_name);
        if dest.exists() {
            failures.push(BulkActionError {
                path: path_str.clone(),
                error: "Destination already exists".to_string(),
            });
            continue;
        }

        let _guard = SuppressionGuard::new(&state.suppressor);

        // Try std::fs::rename first, fallback to copy via fs_extra if cross-device
        if let Err(e) = fs::rename(path, &dest) {
            log::warn!("Rename failed (cross-device?): {}", e);
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = false;
            options.overwrite = false;
            if fs_extra::dir::move_dir(path, target, &options).is_ok() {
                success.push(path_str.to_string());
            } else {
                failures.push(BulkActionError {
                    path: path_str.clone(),
                    error: format!("Failed to move: {}", e),
                });
            }
        } else {
            success.push(path_str.to_string());
        }
    }

    Ok(BulkResult { success, failures })
}

fn handle_archive_import(
    state: &WatcherState,
    path: &Path,
    target: &Path,
    db: &Option<crate::services::scanner::deep_matcher::MasterDb>,
    path_str: &str,
    success: &mut Vec<String>,
    failures: &mut Vec<BulkActionError>,
) {
    let _guard = SuppressionGuard::new(&state.suppressor);

    match extract_archive(path, target, None, false) {
        Ok(result) => {
            if !result.success {
                failures.push(BulkActionError {
                    path: path_str.to_string(),
                    error: result
                        .error
                        .unwrap_or_else(|| "Unknown extraction error".into()),
                });
                return;
            }

            let extracted_path = Path::new(&result.dest_path);
            let folder_name = extracted_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let final_path = if !folder_name.starts_with(DISABLED_PREFIX) {
                let new_path = target.join(format!("{}{}", DISABLED_PREFIX, folder_name));
                if fs::rename(extracted_path, &new_path).is_ok() {
                    new_path
                } else {
                    extracted_path.to_path_buf()
                }
            } else {
                extracted_path.to_path_buf()
            };

            if let Some(master_db) = db {
                match crate::services::scanner::core::organizer::organize_mod(
                    &final_path,
                    target,
                    master_db,
                ) {
                    Ok(res) => success.push(res.new_path.to_string_lossy().to_string()),
                    Err(e) => {
                        log::warn!("Smart Organization failed: {}", e);
                        success.push(final_path.to_string_lossy().to_string());
                    }
                }
            } else {
                success.push(final_path.to_string_lossy().to_string());
            }
        }
        Err(e) => failures.push(BulkActionError {
            path: path_str.to_string(),
            error: e,
        }),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IngestResult {
    pub moved: Vec<String>,
    pub skipped: Vec<String>,
    pub not_dirs: Vec<String>,
    pub sync: crate::services::scanner::sync::SyncResult,
}

#[tauri::command]
pub async fn ingest_dropped_folders(
    _app: tauri::AppHandle,
    _pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    paths: Vec<String>,
    mods_path: String,
    _game_id: String,
    _game_name: String,
    _game_type: String,
) -> Result<IngestResult, String> {
    let _lock = op_lock.acquire().await?;
    let target = Path::new(&mods_path);

    if !target.exists() || !target.is_dir() {
        return Err(format!("Mods path does not exist: {mods_path}"));
    }

    let mut moved = Vec::new();
    let mut skipped = Vec::new();
    let mut not_dirs = Vec::new();

    let _guard = SuppressionGuard::new(&state.suppressor);

    for src_str in &paths {
        let src = Path::new(src_str);
        if !src.is_dir() {
            not_dirs.push(src_str.clone());
            continue;
        }

        let basename = match src.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => {
                skipped.push(src_str.clone());
                continue;
            }
        };

        let dest = target.join(&basename);
        if dest.exists() {
            skipped.push(basename);
            continue;
        }

        if fs::rename(src, &dest).is_ok() {
            moved.push(basename);
        } else {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = false;
            options.overwrite = false;

            if fs_extra::dir::move_dir(src, target, &options).is_ok() {
                moved.push(basename);
            } else {
                skipped.push(basename);
            }
        }
    }

    drop(_guard);

    let sync_result = crate::services::scanner::sync::SyncResult {
        total_scanned: moved.len(),
        new_mods: 0,
        updated_mods: 0,
        deleted_mods: 0,
        new_objects: 0,
    };

    Ok(IngestResult {
        moved,
        skipped,
        not_dirs,
        sync: sync_result,
    })
}
