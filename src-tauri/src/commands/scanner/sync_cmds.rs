//! Commands related to syncing the DB with the file system.

use crate::services::scanner::core::types;
use crate::services::scanner::deep_matcher::MasterDb;
use std::path::Path;
use tauri::{ipc::Channel, Manager, State};

/// Sync the database with the filesystem.
/// Scans the folder, updates existing mods, finds new ones, detects objects, and removes deleted ones.
///
/// # Covers: US-3.5 (Sync)
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn sync_database_cmd(
    app: tauri::AppHandle,
    game_id: String,
    game_name: String,
    game_type: String,
    mods_path: String,
    db_json: String,
    pool: State<'_, sqlx::SqlitePool>,
    on_progress: Channel<types::ScanEvent>,
) -> Result<crate::services::scanner::sync::SyncResult, String> {
    use crate::services::scanner::sync;

    let mods = Path::new(&mods_path);
    if !mods.exists() {
        return Err(format!("Mods path does not exist: {}", mods_path));
    }

    let master_db = MasterDb::from_json(&db_json)?;
    let resource_dir = app.path().resource_dir().ok();

    // 1. Run the scanning/matching phase
    let preview_items = sync::scan_preview(
        &pool,
        &game_id,
        mods,
        &master_db,
        resource_dir.as_deref(),
        Some(on_progress),
        None,
    )
    .await?;

    // 2. Convert preview items to confirmed items without any manual skips
    let confirmed_items = preview_items
        .into_iter()
        .map(|item| sync::ConfirmedScanItem {
            folder_path: item.folder_path,
            display_name: item.display_name,
            is_disabled: item.is_disabled,
            matched_object: item.matched_object,
            object_type: item.object_type,
            thumbnail_path: item.thumbnail_path,
            tags_json: item.tags_json,
            metadata_json: item.metadata_json,
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
    sync::commit_scan_results(
        &pool,
        &game_id,
        &game_name,
        &game_type,
        &mods_path,
        confirmed_items,
        resource_dir.as_deref(),
        &keywords,
    )
    .await
}

/// Phase 1: Scan folders + run Deep Matcher, return preview without writing to DB.
/// Frontend shows a review modal for the user to confirm/override matches.
///
/// # Covers: US-2.3 (Review & Organize UI)
#[tauri::command]
pub async fn scan_preview_cmd(
    app: tauri::AppHandle,
    game_id: String,
    mods_path: String,
    db_json: String,
    pool: State<'_, sqlx::SqlitePool>,
    on_progress: Channel<types::ScanEvent>,
    specific_paths: Option<Vec<String>>,
) -> Result<Vec<crate::services::scanner::sync::ScanPreviewItem>, String> {
    use crate::services::scanner::sync;

    let mods = Path::new(&mods_path);
    if !mods.exists() {
        return Err(format!("Mods path does not exist: {}", mods_path));
    }

    let master_db = MasterDb::from_json(&db_json)?;
    let resource_dir = app.path().resource_dir().ok();

    let optional_paths = specific_paths.map(|paths| {
        paths
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect::<Vec<_>>()
    });

    sync::scan_preview(
        &pool,
        &game_id,
        mods,
        &master_db,
        resource_dir.as_deref(),
        Some(on_progress),
        optional_paths,
    )
    .await
}

/// Phase 2: Commit user-confirmed scan results to DB.
/// Called after the user reviews and confirms/overrides matches in the review modal.
///
/// # Covers: US-2.3 (Review & Organize UI — Confirm)
#[tauri::command]
pub async fn commit_scan_cmd(
    app: tauri::AppHandle,
    game_id: String,
    game_name: String,
    game_type: String,
    mods_path: String,
    items: Vec<crate::services::scanner::sync::ConfirmedScanItem>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<crate::services::scanner::sync::SyncResult, String> {
    use crate::services::scanner::sync;

    let resource_dir = app.path().resource_dir().ok();

    let keywords = app
        .state::<crate::services::config::ConfigService>()
        .get_settings()
        .safe_mode
        .keywords;

    sync::commit_scan_results(
        &pool,
        &game_id,
        &game_name,
        &game_type,
        &mods_path,
        items,
        resource_dir.as_deref(),
        &keywords,
    )
    .await
}

/// Compute percentage scores for a specific batch of candidates against a folder.
/// Used by the Scan Review Modal to lazy-load accurate matching percentages.
///
/// # Covers: US-2.3 (Review & Organize UI — Lazy Scoring)
#[tauri::command]
pub async fn score_candidates_batch_cmd(
    folder_path: String,
    candidate_names: Vec<String>,
    db_json: String,
) -> Result<std::collections::HashMap<String, u8>, String> {
    use crate::services::scanner::sync;

    let master_db = MasterDb::from_json(&db_json)?;

    // Make CPU-bound work non-blocking
    let res = tauri::async_runtime::spawn_blocking(move || {
        sync::score_candidates_batch(&folder_path, &master_db, candidate_names)
    })
    .await
    .map_err(|e| format!("Batch scoring task panicked: {}", e))?;

    Ok(res)
}

#[tauri::command]
pub async fn list_folder_entries_cmd(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_path: String,
    game_id: String,
) -> Result<Vec<crate::services::scanner::folder_entries::FolderEntry>, String> {
    crate::services::scanner::folder_entries::list_folder_entries(
        pool.inner(),
        &game_id,
        &folder_path,
    )
    .await
}

#[cfg(test)]
#[path = "tests/sync_cmds_tests.rs"]
mod tests;
