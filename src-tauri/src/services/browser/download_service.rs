use crate::domain::errors::BrowserError;
use chrono::Utc;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::repo::browser_repo;
use crate::services::browser::{download_handler, import_service};

/// DTO for the frontend download list. Defined in `repo::browser_repo`; re-exported
/// so existing `download_service::BrowserDownloadDto` users keep compiling.
pub use crate::domain::browser::BrowserDownloadDto;

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
    let now = now_stamp();

    browser_repo::insert_download(db, &id, session_id, filename, source_url, file_path, &now)
        .await?;

    Ok(id)
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

    browser_repo::update_status(
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
    Ok(browser_repo::list_downloads(db).await?)
}

/// Delete a download record and optionally the file on disk.
pub async fn delete_download(
    db: &SqlitePool,
    download_id: &str,
    delete_file: bool,
) -> Result<(), BrowserError> {
    if delete_file {
        let path = browser_repo::get_file_path(db, download_id)
            .await
            .ok()
            .flatten();

        if let Some(p) = path {
            crate::services::fs_utils::recycle_bin::move_path_to_recycle_bin(std::path::Path::new(
                &p,
            ))
            .map_err(|error| BrowserError::Io(error.to_string()))?;
        }
    }

    Ok(browser_repo::delete_download(db, download_id).await?)
}

/// Cancel a download: abort the in-flight transfer when one is running,
/// otherwise mark the stale record `canceled` (and optionally drop the file).
pub async fn cancel_download(
    db: &SqlitePool,
    download_id: &str,
    delete_file: Option<bool>,
) -> Result<(), BrowserError> {
    if download_handler::request_cancel(download_id) {
        return Ok(());
    }

    update_status(db, download_id, "canceled", None, None, None, None).await?;
    if delete_file.unwrap_or(false) {
        delete_download(db, download_id, true).await?;
    }
    Ok(())
}

/// Remove all downloads with status `imported`.
pub async fn clear_imported(db: &SqlitePool) -> Result<u64, BrowserError> {
    Ok(browser_repo::delete_imported(db).await?)
}

/// Remove old downloads that exceed the retention period.
pub async fn clear_old_downloads(db: &SqlitePool) -> Result<u64, BrowserError> {
    let retention = browser_repo::get_retention_days(db)
        .await
        .ok()
        .flatten()
        .unwrap_or(30);

    Ok(browser_repo::delete_older_than(db, retention).await?)
}

/// Called by `browser_service` when the download `Finished` event fires.
/// Updates the DB record and optionally triggers the Smart Import pipeline.
pub async fn on_download_finished(
    db: &SqlitePool,
    app: &AppHandle,
    source_url: &str,
    file_path: Option<&str>,
    success: bool,
    tab_label: &str,
) -> Result<(), BrowserError> {
    // Find the download by source_url + tab_label heuristic (most recent requested)
    let row = browser_repo::find_active_by_url(db, source_url).await?;

    let (download_id, session_id) = match row {
        Some(r) => (r.id, r.session_id),
        None => {
            log::warn!("No download record found for URL: {source_url} (tab: {tab_label})");
            return Ok(());
        }
    };

    if success {
        update_status(db, &download_id, "finished", None, None, None, file_path).await?;

        // Emit status update event
        let _ = app.emit(
            "browser:download-status",
            serde_json::json!({
                "id": download_id,
                "status": "finished",
                "file_path": file_path,
            }),
        );

        // Auto-import if enabled
        let auto_import: bool = browser_repo::get_setting(db, "auto_import")
            .await
            .ok()
            .flatten()
            .map(|v: String| v != "false")
            .unwrap_or(true);

        if auto_import {
            if let Some(path) = file_path {
                if let Err(e) = import_service::queue_import_job(
                    db,
                    app,
                    &download_id,
                    session_id.as_deref(),
                    path,
                )
                .await
                {
                    log::error!("Auto-import queue failed: {e}");
                }
            }
        }
    } else {
        update_status(
            db,
            &download_id,
            "failed",
            None,
            None,
            Some("Download failed"),
            None,
        )
        .await?;
        let _ = app.emit(
            "browser:download-status",
            serde_json::json!({
                "id": download_id,
                "status": "failed",
            }),
        );
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/download_service_tests.rs"]
mod tests;
