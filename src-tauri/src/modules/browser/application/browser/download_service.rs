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
    game_id: &str,
    session_id: Option<&str>,
    filename: &str,
    source_url: &str,
    file_path: &str,
) -> Result<String, BrowserError> {
    let id = Uuid::new_v4().to_string();
    create_download_with_id(db, &id, game_id, session_id, filename, source_url, file_path, 0, None).await?;
    Ok(id)
}

/// Insert a new `requested` row with an id reserved by the queue registry.
/// Reserving first makes queue limits and duplicate URL checks atomic in memory.
pub async fn create_download_with_id(
    db: &SqlitePool,
    id: &str,
    game_id: &str,
    session_id: Option<&str>,
    filename: &str,
    source_url: &str,
    file_path: &str,
    queue_order: i64,
    tab_label: Option<&str>,
) -> Result<(), BrowserError> {
    let now = now_stamp();

    browser::insert_download(
        db,
        browser::NewDownloadRow {
            id,
            game_id,
            session_id,
            filename,
            source_url,
            file_path,
            queue_order,
            started_at: &now,
            tab_label,
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

/// Update state for a live WebView2 download. `can_resume` is only meaningful
/// while the originating WebView remains alive.
#[allow(clippy::too_many_arguments)]
pub async fn update_native_status(
    db: &SqlitePool,
    download_id: &str,
    status: &str,
    bytes_received: Option<i64>,
    bytes_total: Option<i64>,
    error_msg: Option<&str>,
    file_path: Option<&str>,
    can_resume: bool,
) -> Result<(), BrowserError> {
    let finished_at = matches!(status, "finished" | "failed" | "canceled").then(now_stamp);
    browser::update_native_status(
        db,
        download_id,
        status,
        bytes_received,
        bytes_total,
        error_msg,
        file_path,
        can_resume,
        finished_at,
    )
    .await?;
    Ok(())
}

/// List all downloads ordered by most recent first.
pub async fn list_downloads(
    db: &SqlitePool,
    game_id: &str,
) -> Result<Vec<BrowserDownloadDto>, BrowserError> {
    Ok(browser::list_downloads(db, game_id).await?)
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
    if download_handler::cancel_native_download(app, download_id)? {
        return Ok(());
    }
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

pub fn pause_download(app: &AppHandle, download_id: &str) -> Result<(), BrowserError> {
    download_handler::pause_native_download(app, download_id)
}

pub fn resume_download(app: &AppHandle, download_id: &str) -> Result<(), BrowserError> {
    download_handler::resume_native_download(app, download_id)
}

/// Reload the tab that issued a non-resumable native download. Many mod hosts
/// use short-lived signed URLs, so this lets the page produce a fresh link.
pub async fn refresh_download_link(
    db: &SqlitePool,
    app: AppHandle,
    download_id: &str,
) -> Result<(), BrowserError> {
    let row = browser::get_retryable_download(db, download_id)
        .await?
        .ok_or_else(|| BrowserError::Download("Only failed or canceled downloads can be refreshed".into()))?;
    let label = row.tab_label.ok_or_else(|| {
        BrowserError::Download("The source tab is no longer available for refresh".into())
    })?;
    browser_service::reload_tab(app, &label).await
}

/// Open the saved source URL in a fresh Discover tab when a server does not
/// support resuming the original download operation.
pub async fn open_download_source(
    db: &SqlitePool,
    app: AppHandle,
    download_id: &str,
) -> Result<String, BrowserError> {
    let row = browser::get_retryable_download(db, download_id)
        .await?
        .ok_or_else(|| BrowserError::Download("Only failed or canceled downloads can be reopened".into()))?;
    let source_url = row
        .source_url
        .ok_or_else(|| BrowserError::Download("The original download URL is unavailable".into()))?;
    browser_service::open_tab(app, db.clone(), source_url, row.session_id).await
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
    let downloads_root = browser_service::get_downloads_root_for_game(app, db, &row.game_id).await;

    download_handler::request_download_confirmation(
        app,
        row.game_id,
        source_url,
        row.filename,
        downloads_root,
        row.session_id,
    )
    .await
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
