//! Commands related to the Deep Match Scanner import pipeline.

use crate::domain::errors::{AppError, ScannerError};
use crate::services::scanner::core::types;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::{Path, PathBuf};
use tauri::{ipc::Channel, Manager, State};

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeepmatchPreviewForObjectsInput {
    pub game_id: String,
    pub mods_path: String,
    pub game_type: i32,
    pub object_ids: Vec<String>,
}

/// Deep Match Scanner command.
/// This path performs canonical matching/import against MasterDB.
/// Do not call from watcher, window focus, or Disk Reconcile triggers.
///
/// # Covers: US-3.5 (Sync)
/// Resolve a mods root the caller named, or report it as missing.
///
/// Three commands spelled this check out inline with their own message.
fn require_mods_dir(mods_path: &str) -> Result<&Path, AppError> {
    let path = Path::new(mods_path);
    if !path.exists() {
        return Err(AppError::Scanner(ScannerError::PathNotFound {
            path: mods_path.to_string(),
        }));
    }
    Ok(path)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing scanner IPC payload stable.
pub async fn deepmatch_scanner_cmd(
    app: tauri::AppHandle,
    state: State<'_, WatcherState>,
    game_id: String,
    game_name: String,
    game_type: String,
    master_db_type: i32,
    mods_path: String,
    preserve_existing_mappings: bool,
    pool: State<'_, sqlx::SqlitePool>,
    on_progress: Channel<types::ScanEvent>,
) -> Result<crate::services::scanner::sync::SyncResult, AppError> {
    use crate::services::scanner::sync;

    let _guard = SuppressionGuard::new(&state.suppressor);

    let mods = require_mods_dir(&mods_path)?;

    let Some(master_db) =
        crate::services::scanner::master_db::get_cached(&app, master_db_type).await?
    else {
        return Err(AppError::Scanner(ScannerError::PathNotFound {
            path: format!("MasterDB for game type {}", master_db_type),
        }));
    };
    let master_db = master_db.as_ref();
    let resource_dir = app.path().resource_dir().ok();

    // 1. Run the scanning/matching phase
    let preview_items = sync::scan_preview(
        &pool,
        &game_id,
        mods,
        master_db,
        resource_dir.as_deref(),
        Some(on_progress),
        None,
    )
    .await?;

    let confirmed_items: Vec<_> = preview_items
        .into_iter()
        .map(|item| sync::ConfirmedScanItem {
            folder_path: item.folder_path,
            display_name: item.display_name,
            is_disabled: item.is_disabled,
            matched_entry_key: item.matched_entry_key,
            matched_alias_name: item.matched_alias_name,
            matched_confidence: Some(f64::from(item.confidence_score) / 100.0),
            matched_reason: item.match_detail,
            object_type: item.object_type,
            thumbnail_path: item.thumbnail_path,
            tags_json: item.tags_json,
            metadata_json: item.metadata_json,
            hash_db_json: item.hash_db_json,
            custom_skins_json: item.custom_skins_json,
            db_thumbnail: item.db_thumbnail,
            skip: false,
            move_from_temp: false,
        })
        .collect();

    let keywords = app
        .state::<crate::services::config::ConfigService>()
        .get_settings()
        .safe_mode
        .keywords;

    // 3. Commit the results to DB
    let result = sync::commit_scan_results(sync::CommitScanRequest {
        pool: &pool,
        game_id: &game_id,
        game_name: &game_name,
        game_type: &game_type,
        mods_path: &mods_path,
        items: confirmed_items,
        resource_dir: resource_dir.as_deref(),
        safe_mode_keywords: &keywords,
        preserve_existing_mappings,
    })
    .await?;

    Ok(result)
}

/// Deep Match Scanner preview.
/// This path runs explicit matching preview without writing to DB.
/// Frontend shows a review modal for the user to confirm/override matches.
///
/// # Covers: US-2.3 (Review & Organize UI)
#[tauri::command]
#[specta::specta]
pub async fn deepmatch_preview_cmd(
    app: tauri::AppHandle,
    game_id: String,
    mods_path: String,
    game_type: i32,
    pool: State<'_, sqlx::SqlitePool>,
    on_progress: Channel<types::ScanEvent>,
    specific_paths: Option<Vec<String>>,
) -> Result<Vec<crate::services::scanner::sync::ScanPreviewItem>, AppError> {
    use crate::services::scanner::sync;

    let mods = require_mods_dir(&mods_path)?;

    let Some(master_db) = crate::services::scanner::master_db::get_cached(&app, game_type).await?
    else {
        return Err(AppError::Scanner(ScannerError::PathNotFound {
            path: format!("MasterDB for game type {}", game_type),
        }));
    };
    let master_db = master_db.as_ref();
    let resource_dir = app.path().resource_dir().ok();

    let optional_paths = specific_paths.map(|paths| {
        paths
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect::<Vec<_>>()
    });

    Ok(sync::scan_preview(
        &pool,
        &game_id,
        mods,
        master_db,
        resource_dir.as_deref(),
        Some(on_progress),
        optional_paths,
    )
    .await?)
}

/// Deep Match Scanner preview for object IDs already selected in the workspace UI.
/// Backend resolves DB paths and filters stale or escaped paths before scanning.
#[tauri::command]
#[specta::specta]
pub async fn deepmatch_preview_for_objects_cmd(
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    input: DeepmatchPreviewForObjectsInput,
    on_progress: Channel<types::ScanEvent>,
) -> Result<Vec<crate::services::scanner::sync::ScanPreviewItem>, AppError> {
    use crate::services::scanner::sync;

    let mods = require_mods_dir(&input.mods_path)?;

    let object_paths =
        resolve_object_preview_paths(&pool, &input.game_id, mods, &input.object_ids).await?;
    if object_paths.is_empty() {
        return Ok(Vec::new());
    }

    let Some(master_db) =
        crate::services::scanner::master_db::get_cached(&app, input.game_type).await?
    else {
        return Err(AppError::Scanner(ScannerError::PathNotFound {
            path: format!("MasterDB for game type {}", input.game_type),
        }));
    };
    let master_db = master_db.as_ref();
    let resource_dir = app.path().resource_dir().ok();

    Ok(sync::scan_preview(
        &pool,
        &input.game_id,
        mods,
        master_db,
        resource_dir.as_deref(),
        Some(on_progress),
        Some(object_paths),
    )
    .await?)
}

async fn resolve_object_preview_paths(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    object_ids: &[String],
) -> Result<Vec<PathBuf>, AppError> {
    if object_ids.is_empty() {
        return Ok(Vec::new());
    }

    let canonical_root = mods_path.canonicalize().map_err(|error| {
        AppError::Scanner(ScannerError::Io(format!(
            "failed to canonicalize mods root: {error}"
        )))
    })?;

    let folder_paths =
        crate::repo::mod_repo::get_folder_paths_by_object_ids(pool, game_id, object_ids).await?;

    let mut resolved_paths = Vec::with_capacity(folder_paths.len());
    for folder_path in folder_paths {
        let raw_path = Path::new(&folder_path);
        let candidate_path = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            canonical_root.join(raw_path)
        };

        let Ok(canonical_candidate) = candidate_path.canonicalize() else {
            continue;
        };

        if !canonical_candidate.is_dir() {
            continue;
        }

        if !canonical_candidate.starts_with(&canonical_root) {
            log::warn!(
                "Skipping object preview path outside mods root: {}",
                canonical_candidate.display()
            );
            continue;
        }

        resolved_paths.push(canonical_candidate);
    }

    Ok(resolved_paths)
}

/// Phase 2: Commit user-confirmed scan results to DB.
/// Called after the user reviews and confirms/overrides matches in the review modal.
///
/// # Covers: US-2.3 (Review & Organize UI — Confirm)
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn commit_scan_cmd(
    app: tauri::AppHandle,
    state: State<'_, WatcherState>,
    game_id: String,
    game_name: String,
    game_type: String,
    mods_path: String,
    items: Vec<crate::services::scanner::sync::ConfirmedScanItem>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<crate::services::scanner::sync::SyncResult, AppError> {
    use crate::services::scanner::sync;

    let _guard = SuppressionGuard::new(&state.suppressor);

    let resource_dir = app.path().resource_dir().ok();

    let keywords = app
        .state::<crate::services::config::ConfigService>()
        .get_settings()
        .safe_mode
        .keywords;

    let result = sync::commit_scan_results(sync::CommitScanRequest {
        pool: &pool,
        game_id: &game_id,
        game_name: &game_name,
        game_type: &game_type,
        mods_path: &mods_path,
        items,
        resource_dir: resource_dir.as_deref(),
        safe_mode_keywords: &keywords,
        preserve_existing_mappings: false,
    })
    .await?;

    Ok(result)
}

/// Compute percentage scores for a specific batch of candidates against a folder.
/// Used by the Scan Review Modal to lazy-load accurate matching percentages.
///
/// # Covers: US-2.3 (Review & Organize UI — Lazy Scoring)
#[tauri::command]
#[specta::specta]
pub async fn score_candidates_batch_cmd(
    folder_path: String,
    candidate_names: Vec<String>,
    game_type: i32,
    app: tauri::AppHandle,
) -> Result<std::collections::HashMap<String, u8>, AppError> {
    use crate::services::scanner::sync;

    let Some(master_db) = crate::services::scanner::master_db::get_cached(&app, game_type).await?
    else {
        return Err(AppError::Scanner(ScannerError::PathNotFound {
            path: format!("MasterDB for game type {}", game_type),
        }));
    };
    // Move the Arc, not the 5 MB behind it.
    let res = tauri::async_runtime::spawn_blocking(move || {
        sync::score_candidates_batch(&folder_path, &master_db, candidate_names)
    })
    .await
    .map_err(|e| AppError::Internal(format!("batch scoring task panicked: {e}")))?;

    Ok(res)
}

#[tauri::command]
#[specta::specta]
pub async fn list_folder_entries_cmd(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_path: String,
    game_id: String,
) -> Result<Vec<crate::services::scanner::folder_entries::FolderEntry>, AppError> {
    Ok(
        crate::services::scanner::folder_entries::list_folder_entries(
            pool.inner(),
            &game_id,
            &folder_path,
        )
        .await?,
    )
}

#[cfg(test)]
#[path = "tests/deepmatch_scanner_cmds_tests.rs"]
mod tests;
