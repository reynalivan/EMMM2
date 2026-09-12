use crate::shared::errors::BrowserError;
use chrono::Utc;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::modules::browser::adapters::sqlite::browser;
use crate::modules::browser::application::browser::{browser_service, download_handler};

/// DTO for the frontend download list. Defined in `repo::browser`; re-exported
/// so existing `download_service::BrowserDownloadDto` users keep compiling.
pub use crate::modules::browser::domain::browser::BrowserDownloadDto;

fn now_stamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Insert a new `requested` download record.
pub async fn create_download(
    db: &SqlitePool,
    session_id: Option<&str>,
    filename: &str,
    source_url: &str,
    file_path: &str,
) -> Result<String, BrowserError> {
    let id = Uuid::new_v4().to_string();
    create_download_with_id(db, &id, session_id, filename, source_url, file_path, 0).await?;
    Ok(id)
}

/// Insert a new `requested` row with an id reserved by the queue registry.
/// Reserving first makes queue limits and duplicate URL checks atomic in memory.
pub async fn create_download_with_id(
    db: &SqlitePool,
    id: &str,
    session_id: Option<&str>,
    filename: &str,
    source_url: &str,
    file_path: &str,
    queue_order: i64,
) -> Result<(), BrowserError> {
    let now = now_stamp();

    browser::insert_download(
        db,
        browser::NewDownloadRow {
            id,
            session_id,
            filename,
            source_url,
            file_path,
            queue_order,
            started_at: &now,
        },
    )
    .await?;

    Ok(())
}

/// Update download status + optional progress fields.
pub async fn update_status(
    db: &SqlitePool,
    download_id: &str,
    status: &str,
    bytes_received: Option<i64>,
    bytes_total: Option<i64>,
    error_msg: Option<&str>,
    file_path: Option<&str>,
) -> Result<(), BrowserError> {
    let finished_at = matches!(status, "finished" | "failed" | "canceled").then(now_stamp);

    browser::update_status(
        db,
        download_id,
        status,
        bytes_received,
        bytes_total,
        error_msg,
        file_path,
        finished_at,
    )
    .await?;
    Ok(())
}

/// List all downloads ordered by most recent first.
pub async fn list_downloads(db: &SqlitePool) -> Result<Vec<BrowserDownloadDto>, BrowserError> {
    Ok(browser::list_downloads(db).await?)
}

/// Delete a download record and optionally the file on disk.
pub async fn delete_download(
    db: &SqlitePool,
    download_id: &str,
    delete_file: bool,
) -> Result<(), BrowserError> {
    if delete_file {
        let path = browser::get_file_path(db, download_id).await.ok().flatten();

        if let Some(p) = path {
            crate::platform::fs::recycle_bin::move_path_to_recycle_bin(std::path::Path::new(&p))
                .map_err(|error| BrowserError::Io(error.to_string()))?;
        }
    }

    Ok(browser::delete_download(db, download_id).await?)
}

/// Cancel a download: abort the in-flight transfer when one is running,
/// otherwise mark the stale record `canceled` (and optionally drop the file).
pub async fn cancel_download(
    db: &SqlitePool,
    download_id: &str,
    delete_file: Option<bool>,
) -> Result<(), BrowserError> {
    if download_handler::request_cancel(download_id).is_some() {
        return Ok(());
    }

    update_status(db, download_id, "canceled", None, None, None, None).await?;
    if delete_file.unwrap_or(false) {
        delete_download(db, download_id, true).await?;
    }
    Ok(())
}

/// Cancel a download and immediately notify the UI if it was still queued.
/// Active transfers report their terminal state from the worker after partial
/// file cleanup has completed.
pub async fn cancel_download_with_feedback(
    db: &SqlitePool,
    app: &AppHandle,
    download_id: &str,
    delete_file: Option<bool>,
) -> Result<(), BrowserError> {
    match download_handler::request_cancel(download_id) {
        Some(download_handler::CancelRequest::InProgress) => Ok(()),
        Some(download_handler::CancelRequest::Queued) => {
            update_status(db, download_id, "canceled", None, None, None, None).await?;
            if delete_file.unwrap_or(false) {
                delete_download(db, download_id, true).await?;
            }
            let _ = app.emit(
                "browser:download-status",
                serde_json::json!({ "id": download_id, "status": "canceled" }),
            );
            Ok(())
        }
        None => {
            cancel_download(db, download_id, delete_file).await?;
            let _ = app.emit(
                "browser:download-status",
                serde_json::json!({ "id": download_id, "status": "canceled" }),
            );
            Ok(())
        }
    }
}

/// Request confirmation for re-downloading a failed or canceled item. A retry
/// remains subject to the same explicit user approval as a fresh browser link.
pub async fn retry_download(
    db: &SqlitePool,
    app: &AppHandle,
    download_id: &str,
) -> Result<(), BrowserError> {
    let row = browser::get_retryable_download(db, download_id)
        .await?
        .ok_or_else(|| {
            BrowserError::Download("Only failed or canceled downloads can be retried".into())
        })?;
    let source_url = row
        .source_url
        .ok_or_else(|| BrowserError::Download("The original download URL is unavailable".into()))?;
    let downloads_root = browser_service::get_downloads_root(app, db).await;

    download_handler::request_download_confirmation(
        app,
        source_url,
        row.filename,
        downloads_root,
        row.session_id,
    )
}

/// Remove old downloads that exceed the retention period.
pub async fn clear_old_downloads(db: &SqlitePool) -> Result<u64, BrowserError> {
    let retention = browser_service::get_retention_days(db).await?;

    Ok(browser::delete_older_than(db, retention).await?)
}

/// Persist one worker's terminal state by the id assigned at queue admission.
/// This deliberately never searches by URL: retrying an identical URL must not
/// let the older worker update the newer row.
pub async fn mark_download_finished(
    db: &SqlitePool,
    app: &AppHandle,
    download_id: &str,
    file_path: &str,
) -> Result<(), BrowserError> {
    update_status(
        db,
        download_id,
        "finished",
        None,
        None,
        None,
        Some(file_path),
    )
    .await?;

    let _ = app.emit(
        "browser:download-status",
        serde_json::json!({
            "id": download_id,
            "status": "finished",
            "file_path": file_path,
        }),
    );

    Ok(())
}

#[cfg(test)]
#[path = "tests/download_service_tests.rs"]
mod tests;
