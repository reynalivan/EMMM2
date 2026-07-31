//! Moving a matched extraction into the workspace and closing out the job.

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use uuid::Uuid;

use crate::repo::browser_repo::{self, ImportJobMatch as MatchResult};
use crate::services::disk_reconcile::emit::emit_internal_disk_reconcile;
use crate::services::scanner::sync::helpers::{
    resolve_or_create_object_target_for_match, ResolveObjectTargetInput,
};

use super::jobs::{emit_status, set_job_status};

pub(super) async fn place_mod(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    extract_dir: &Path,
    match_result: &MatchResult,
    selected_object_id: Option<&str>,
) -> Result<(), String> {
    set_job_status(db, job_id, "placing", None).await?;

    let game_id: Option<String> = browser_repo::get_job_game_id(db, job_id)
        .await
        .ok()
        .flatten();

    let game_id = game_id.ok_or("No game_id set on import job — cannot place mod")?;

    let mods_path = crate::repo::game_repo::get_configured_mods_path(db, &game_id)
        .await
        .map_err(|e| format!("Game not found: {e}"))?
        .ok_or_else(|| "Game not found: mods_path is not configured".to_string())?;

    let mod_name = extract_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("ImportedMod_{}", Uuid::new_v4().as_simple()));

    let mut tx = db.begin().await.map_err(|error| error.to_string())?;
    let mut shell_objects_created = 0;

    let selected_target = if let Some(object_id) = selected_object_id {
        let folder_path = crate::repo::object_repo::get_folder_path_conn(&mut tx, object_id)
            .await
            .map_err(|error| format!("Failed to resolve selected object folder: {error}"))?;

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
                physical_name_hint: &mod_name,
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
        .await?
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
        std::fs::create_dir_all(&target_parent)
            .map_err(|error| format!("Failed to create object shell folder: {error}"))?;
    }

    let dest = target_parent.join(format!("{}{}", crate::DISABLED_PREFIX, mod_name));

    // Collision guard
    let dest = resolve_collision(dest);

    std::fs::rename(extract_dir, &dest)
        .or_else(|_| {
            // cross-drive fallback
            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(extract_dir, &dest)
        })
        .map_err(|e| format!("Failed to place mod: {e}"))?;

    let dest_str = dest.to_string_lossy().to_string();

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
        .await
        .map_err(|e| format!("Failed to persist browser import canonical relation: {e}"))?;
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

    tx.commit().await.map_err(|error| error.to_string())?;

    emit_internal_disk_reconcile(app, db, &game_id, vec![dest_str.clone()]).await?;

    emit_status(
        app,
        job_id,
        "done",
        Some(serde_json::json!({ "placed_path": dest_str })),
    );
    Ok(())
}

fn resolve_collision(dest: PathBuf) -> PathBuf {
    if !dest.exists() {
        return dest;
    }
    let base = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod".to_string());
    let parent = dest.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut n = 2u32;
    loop {
        let candidate = parent.join(format!("{base} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}
