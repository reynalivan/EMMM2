//! Enqueueing import jobs and kicking off their pipeline runs.

use chrono::Utc;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::repo::browser_repo;

use super::jobs::set_job_status;
use super::pipeline::run_pipeline;

/// Max concurrent import pipelines (BLAKE3 hash + stage copy + archive extract).
/// A bulk import of 20 downloads would otherwise thrash one disk with 20 runs.
/// ponytail: one global cap; split per drive only if libraries ever span disks.
static PIPELINE_SEMAPHORE: Semaphore = Semaphore::const_new(2);

/// Enqueue a new import job and immediately spawn the pipeline.
pub async fn queue_import_job(
    db: &SqlitePool,
    app: &AppHandle,
    download_id: &str,
    session_id: Option<&str>,
    archive_path: &str,
) -> Result<String, String> {
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    // Determine game_id from session if available
    let game_id: Option<String> = match session_id {
        Some(sid) => browser_repo::get_session_game_id(db, sid)
            .await
            .ok()
            .flatten(),
        None => None,
    };

    browser_repo::insert_job(
        db,
        &job_id,
        download_id,
        game_id.as_deref(),
        archive_path,
        &now,
    )
    .await
    .map_err(|e| format!("DB insert import_job failed: {e}"))?;

    spawn_pipeline(db, app, &job_id, archive_path, "Import pipeline error");

    Ok(job_id)
}

/// Bulk-queue import jobs for a list of download IDs (from Download Manager multi-select).
pub async fn bulk_queue_imports(
    db: &SqlitePool,
    app: &AppHandle,
    download_ids: &[String],
    game_id: &str,
) -> Result<Vec<String>, String> {
    let mut job_ids = Vec::with_capacity(download_ids.len());

    for dl_id in download_ids {
        let row = browser_repo::get_finished_for_import(db, dl_id)
            .await
            .map_err(|e| format!("DB error: {e}"))?;

        let Some(r) = row else { continue };
        let Some(file_path) = r.file_path else {
            continue;
        };

        // Override game_id
        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        browser_repo::insert_job(db, &job_id, dl_id, Some(game_id), &file_path, &now)
            .await
            .map_err(|e| format!("DB insert failed: {e}"))?;

        spawn_pipeline(db, app, &job_id, &file_path, "Bulk import pipeline error");

        job_ids.push(job_id);
    }

    Ok(job_ids)
}

/// Run the pipeline off-thread, waiting for a `PIPELINE_SEMAPHORE` permit first;
/// the job stays `queued` in the meantime. A failure marks it `failed` and tells the UI.
fn spawn_pipeline(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    archive_path: &str,
    error_context: &'static str,
) {
    let db_c = db.clone();
    let app_c = app.clone();
    let job_id_c = job_id.to_string();
    let archive = archive_path.to_string();
    tauri::async_runtime::spawn(async move {
        let outcome = match PIPELINE_SEMAPHORE.acquire().await {
            Ok(_permit) => run_pipeline(&db_c, &app_c, &job_id_c, &archive).await,
            Err(e) => Err(format!("Import queue closed: {e}")),
        };

        if let Err(e) = outcome {
            log::error!("{error_context} for job {job_id_c}: {e}");
            let _ = set_job_status(&db_c, &job_id_c, "failed", Some(&e)).await;
            let _ = app_c.emit(
                "import:job-update",
                serde_json::json!({
                    "job_id": job_id_c,
                    "status": "failed",
                    "error": e,
                }),
            );
        }
    });
}
