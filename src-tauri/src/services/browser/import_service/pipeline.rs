//! The queued → extracted → matched → placed run for a single import job.

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::repo::browser_repo;

use super::jobs::{emit_status, set_job_status};
use super::matching::try_deep_match;
use super::placement::place_mod;

pub(super) async fn run_pipeline(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    archive_path: &str,
) -> Result<(), String> {
    let archive = PathBuf::from(archive_path);

    // -- Step 1: Validate extension is allowed --
    validate_extension(db, &archive).await?;

    // -- Step 2: Hash (BLAKE3) + dedup check --
    emit_status(app, job_id, "extracting", None);
    set_job_status(db, job_id, "extracting", None).await?;

    let hash = hash_file(&archive)?;

    // Check dedup
    let existing = browser_repo::count_done_with_hash(db, &hash)
        .await
        .unwrap_or(0);

    if existing > 0 {
        browser_repo::mark_duplicate(db, job_id).await.ok();
        // Inform frontend — user must decide what to do with duplicate
        emit_status(
            app,
            job_id,
            "needs_review",
            Some(serde_json::json!({
                "reason": "duplicate",
                "archive_hash": hash,
            })),
        );
        return set_job_status(db, job_id, "needs_review", None).await;
    }

    // Store hash
    browser_repo::set_archive_hash(db, job_id, &hash).await.ok();

    // -- Step 3: Stage (copy to staging dir) --
    let staging_path = stage_archive(app, job_id, &archive).await?;
    let staging_path_str = staging_path.to_string_lossy().to_string();
    browser_repo::set_staging_path(db, job_id, &staging_path_str)
        .await
        .ok();

    // -- Step 4: Extract --
    let extract_dir = staging_path.parent().unwrap().join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extract dir: {e}"))?;

    let password: Option<&str> = None; // No password UI yet; extend later
    crate::services::mods::archive::extract_archive(
        &staging_path,
        &extract_dir,
        password,
        true,
        None,
        None,
        false,
        false,
        None,
    )?;

    // -- Step 5: Validate (check for at least one .ini file) --
    let ini_count = count_ini_files(&extract_dir);

    if ini_count == 0 {
        return Err(
            "No .ini files found in archive — this does not appear to be a valid 3DMigoto mod."
                .to_string(),
        );
    }

    // -- Step 6: Deep Match Scanner --
    set_job_status(db, job_id, "matching", None).await?;
    emit_status(app, job_id, "matching", None);

    // Load game_id for this job
    let game_id: Option<String> = browser_repo::get_job_game_id(db, job_id)
        .await
        .ok()
        .flatten();

    let match_result = try_deep_match(app, &extract_dir, game_id.as_deref()).await;

    // Store match result
    if let Some(ref m) = match_result {
        let _ = browser_repo::set_match_result(db, job_id, m).await;
    }

    let confidence = match_result.as_ref().map(|m| m.confidence).unwrap_or(0.0);
    if confidence < 0.70 {
        // Needs manual review
        set_job_status(db, job_id, "needs_review", None).await?;
        emit_status(
            app,
            job_id,
            "needs_review",
            match_result.as_ref().map(|m| {
                serde_json::json!({
                    "category": m.category,
                    "entry_key": m.entry_key,
                    "alias_name": m.alias_name,
                    "confidence": m.confidence,
                    "reason": m.reason,
                })
            }),
        );
        return Ok(());
    }

    // -- Step 7: Place --
    place_mod(db, app, job_id, &extract_dir, &match_result.unwrap(), None).await
}

/// Number of `.ini` files anywhere under `dir` — the 3DMigoto sanity signal.
pub(super) fn count_ini_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "ini").unwrap_or(false))
        .count()
}

async fn validate_extension(db: &SqlitePool, path: &Path) -> Result<(), String> {
    let allowed_raw: String = browser_repo::get_setting(db, "allowed_extensions")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| ".zip,.7z,.rar,.tar,.gz".to_string());

    let allowed: Vec<&str> = allowed_raw.split(',').map(str::trim).collect();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    if allowed.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "Extension '{ext}' is not in the allowed list: {allowed_raw}"
        ))
    }
}

fn hash_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("Cannot read file for hashing: {e}"))?;
    Ok(blake3::hash(&data).to_hex().to_string())
}

async fn stage_archive(app: &AppHandle, job_id: &str, archive: &Path) -> Result<PathBuf, String> {
    let staging_root = match app.path().app_data_dir() {
        Ok(d) => d.join("staging"),
        Err(_) => PathBuf::from("staging"),
    };
    let job_dir = staging_root.join(job_id);
    std::fs::create_dir_all(&job_dir).map_err(|e| format!("Cannot create staging dir: {e}"))?;

    let filename = archive.file_name().ok_or("Archive has no filename")?;
    let dest = job_dir.join(filename);
    std::fs::copy(archive, &dest).map_err(|e| format!("Cannot copy to staging: {e}"))?;
    Ok(dest)
}
