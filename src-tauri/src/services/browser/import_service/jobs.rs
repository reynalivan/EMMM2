//! Job listing, status transitions, and the manual review / cancel decisions.

use chrono::Utc;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use crate::repo::browser_repo::{self, ImportJobMatch as MatchResult};

use super::placement::place_mod;

/// DTO returned to the frontend for import queue display. Defined in
/// `repo::browser_repo`; re-exported so existing users keep compiling.
pub use crate::repo::browser_repo::ImportJobDto;

/// Return all active (non-canceled) import jobs ordered by most recent first.
pub async fn list_jobs(db: &SqlitePool) -> Result<Vec<ImportJobDto>, String> {
    browser_repo::list_active_jobs(db)
        .await
        .map_err(|e| format!("DB list jobs failed: {e}"))
}

/// Manual confirmation for a needs_review job.
/// Sets game_id, category, object_id, then resumes pipeline (place step).
pub async fn confirm_review(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    game_id: &str,
    category: &str,
    object_id: Option<&str>,
) -> Result<(), String> {
    browser_repo::apply_review_decision(db, job_id, game_id, category)
        .await
        .map_err(|e| format!("DB confirm_review failed: {e}"))?;

    // Resume placement
    let archive_opt: Option<String> = browser_repo::require_staging_path(db, job_id)
        .await
        .map_err(|e| format!("Job not found: {e}"))?;

    let archive = archive_opt.ok_or_else(|| "No staging_path for job".to_string())?;

    let extract_dir = PathBuf::from(&archive).parent().unwrap().join("extracted");

    let mut match_result = load_job_match_result(db, job_id).await?;
    match_result.category = Some(category.to_string());
    if match_result.reason.is_none() {
        match_result.reason = Some("User confirmed".to_string());
    }
    if match_result.confidence <= 0.0 {
        match_result.confidence = 1.0;
    }

    place_mod(db, app, job_id, &extract_dir, &match_result, object_id).await
}

/// Cancel a job and clean up its staging folder.
pub async fn cancel_job(db: &SqlitePool, job_id: &str) -> Result<(), String> {
    let staging: Option<String> = browser_repo::get_staging_path(db, job_id)
        .await
        .ok()
        .flatten();

    if let Some(p) = staging {
        let staging_dir = PathBuf::from(&p)
            .parent()
            .map(|d| d.to_path_buf())
            .unwrap_or_default();
        if staging_dir.exists() {
            let _ = std::fs::remove_dir_all(&staging_dir);
        }
    }

    browser_repo::mark_canceled(db, job_id)
        .await
        .map_err(|e| format!("DB cancel_job failed: {e}"))
}

async fn load_job_match_result(db: &SqlitePool, job_id: &str) -> Result<MatchResult, String> {
    browser_repo::load_match_result(db, job_id)
        .await
        .map_err(|e| format!("Failed to load import job match result: {e}"))
}

pub(super) fn emit_status(
    app: &AppHandle,
    job_id: &str,
    status: &str,
    extra: Option<serde_json::Value>,
) {
    let mut payload = serde_json::json!({ "job_id": job_id, "status": status });
    if let Some(serde_json::Value::Object(map)) = extra {
        if let serde_json::Value::Object(ref mut p) = payload {
            p.extend(map);
        }
    }
    let _ = app.emit("import:job-update", payload);
}

pub(super) async fn set_job_status(
    db: &SqlitePool,
    job_id: &str,
    status: &str,
    error_msg: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    browser_repo::set_status(db, job_id, status, error_msg, &now)
        .await
        .map_err(|e| format!("DB update import_job status failed: {e}"))
}
