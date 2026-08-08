//! Moving a matched extraction into the workspace and closing out the job.

use crate::domain::errors::BrowserError;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

use crate::repo::browser_repo::{self, ImportJobMatch as MatchResult};
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::sync::helpers::{
    resolve_or_create_object_target_for_match, ResolveObjectTargetInput,
};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use tauri::Manager;

use super::jobs::{emit_status, set_job_status};

/// Move extracted mod roots into the workspace and close out the import job.
///
/// `mod_roots` are the folders `extract_archive` produced — each already
/// carries the mod's real name from inside the archive.
pub(super) async fn place_mod(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    mod_roots: &[PathBuf],
    match_result: &MatchResult,
    selected_object_id: Option<&str>,
) -> Result<(), BrowserError> {
    set_job_status(db, job_id, "placing", None).await?;

    // This writes into the mods root, so it must serialize against bulk
    // operations and must not wake the watcher — the manual import path
    // (`mod_import_cmds`) takes both guards for the same reason.
    let op_lock = app
        .try_state::<OperationLock>()
        .ok_or_else(|| BrowserError::Import("OperationLock not available".to_string()))?;
    let _lock = op_lock
        .acquire()
        .await
        .map_err(|error| BrowserError::Import(error.to_string()))?;

    let watcher = app
        .try_state::<WatcherState>()
        .ok_or_else(|| BrowserError::Import("WatcherState not available".to_string()))?;
    let _suppression = SuppressionGuard::new(&watcher.suppressor);

    let game_id: Option<String> = browser_repo::get_job_game_id(db, job_id)
        .await
        .ok()
        .flatten();

    let game_id = game_id.ok_or_else(|| BrowserError::JobIncomplete {
        job_id: job_id.to_string(),
        field: "game_id".to_string(),
    })?;

    let mods_path = crate::repo::game_repo::get_configured_mods_path(db, &game_id)
        .await?
        .ok_or_else(|| {
            BrowserError::Import(format!("game '{game_id}' has no mods_path configured"))
        })?;

    // The primary root names the object-resolution hint; every root is placed.
    let primary_name = mod_root_name(mod_roots.first());

    let mut tx = db.begin().await?;
    let mut shell_objects_created = 0;

    let selected_target = if let Some(object_id) = selected_object_id {
        let folder_path = crate::repo::object_repo::get_folder_path_conn(&mut tx, object_id)
            .await
            .map_err(|error| {
                BrowserError::Import(format!("failed to resolve selected object folder: {error}"))
            })?;

        folder_path.map(
            |path| crate::services::scanner::sync::helpers::ResolvedObjectTarget {
                object_id: object_id.to_string(),
                folder_path: path,
            },
        )
    } else {
        None
    };

    let match_target = if selected_target.is_none() {
        resolve_or_create_object_target_for_match(
            &mut tx,
            ResolveObjectTargetInput {
                game_id: &game_id,
                mods_path: &mods_path,
                physical_name_hint: &primary_name,
                matched_entry_key: match_result.entry_key.as_deref(),
                object_type: match_result.category.as_deref().unwrap_or("Other"),
                db_thumbnail: None,
                db_tags_json: "[]",
                db_metadata_json: "{}",
                db_hash_db_json: None,
                db_custom_skins_json: None,
            },
            &mut shell_objects_created,
        )
        .await
        .map_err(|error| BrowserError::Import(error.to_string()))?
    } else {
        None
    };

    let resolved_target = selected_target.or(match_target);
    let target_object_folder = resolved_target
        .as_ref()
        .map(|target| target.folder_path.clone())
        .unwrap_or_else(|| "Other".to_string());
    let target_object_id = resolved_target
        .as_ref()
        .map(|target| target.object_id.clone());

    let target_parent = PathBuf::from(&mods_path).join(&target_object_folder);
    if !target_parent.exists() {
        std::fs::create_dir_all(&target_parent)?;
    }

    let mut placed_paths: Vec<String> = Vec::with_capacity(mod_roots.len());
    for root in mod_roots {
        let dest = crate::services::mods::arrival::land_disabled(root, &target_parent)
            .map_err(|error| BrowserError::Import(error.to_string()))?;

        placed_paths.push(dest.to_string_lossy().to_string());
    }

    let dest_str = placed_paths.first().cloned().ok_or_else(|| {
        BrowserError::Import("extraction produced no mod folders to place".to_string())
    })?;

    if let Some(object_id) = target_object_id.as_deref() {
        crate::repo::object_repo::apply_canonical_match(
            &mut *tx,
            object_id,
            match_result.entry_key.as_deref(),
            match_result.alias_name.as_deref(),
            Some(match_result.confidence),
            match_result.reason.as_deref(),
            Some("browser_import"),
        )
        .await?;
    }

    browser_repo::set_placed_done(&mut tx, job_id, &dest_str)
        .await
        .ok();

    // Mark the linked download as imported
    if let Some(dl_id) = browser_repo::get_download_id(&mut tx, job_id)
        .await
        .ok()
        .flatten()
    {
        browser_repo::mark_imported(&mut tx, &dl_id).await.ok();
    }

    tx.commit().await?;

    emit_internal_disk_reconcile(app, db, &game_id, placed_paths.clone())
        .await
        .map_err(|error| BrowserError::Import(error.to_string()))?;

    emit_status(
        app,
        job_id,
        "done",
        Some(serde_json::json!({
            "placed_path": dest_str,
            "placed_paths": placed_paths,
        })),
    );
    Ok(())
}

/// Folder name of an extracted mod root, with a unique fallback so a nameless
/// path cannot collapse two imports onto one destination.
fn mod_root_name(root: Option<&PathBuf>) -> String {
    root.and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("ImportedMod_{}", Uuid::new_v4().as_simple()))
}
