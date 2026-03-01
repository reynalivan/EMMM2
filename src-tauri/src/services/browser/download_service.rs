use chrono::Utc;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::services::browser::import_service;

/// DTO for the frontend download list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct BrowserDownloadDto {
    pub id: String,
    pub session_id: Option<String>,
    pub filename: String,
    pub file_path: Option<String>,
    pub source_url: Option<String>,
    pub status: String,
    pub bytes_total: Option<i64>,
    pub bytes_received: i64,
    pub error_msg: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

/// Insert a new `requested` download record.
pub async fn create_download(
    db: &SqlitePool,
    session_id: Option<&str>,
    filename: &str,
    source_url: &str,
    file_path: &str,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    sqlx::query!(
        r#"INSERT INTO browser_downloads
           (id, session_id, filename, file_path, source_url, status, bytes_received, started_at)
           VALUES (?, ?, ?, ?, ?, 'requested', 0, ?)"#,
        id,
        session_id,
        filename,
        file_path,
        source_url,
        now
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB insert failed: {e}"))?;

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
) -> Result<(), String> {
    let finished_at: Option<String> =
        if status == "finished" || status == "failed" || status == "canceled" {
            Some(Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string())
        } else {
            None
        };

    sqlx::query!(
        r#"UPDATE browser_downloads SET
            status         = ?,
            bytes_received = COALESCE(?, bytes_received),
            bytes_total    = COALESCE(?, bytes_total),
            error_msg      = ?,
            file_path      = COALESCE(?, file_path),
            finished_at    = COALESCE(?, finished_at)
          WHERE id = ?"#,
        status,
        bytes_received,
        bytes_total,
        error_msg,
        file_path,
        finished_at,
        download_id
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB update failed: {e}"))?;
    Ok(())
}

/// List all downloads ordered by most recent first.
pub async fn list_downloads(db: &SqlitePool) -> Result<Vec<BrowserDownloadDto>, String> {
    let rows = sqlx::query_as::<_, BrowserDownloadDto>(
        r#"SELECT id, session_id, filename, file_path, source_url,
                  status, bytes_total, bytes_received, error_msg,
                  started_at, finished_at
           FROM browser_downloads
           ORDER BY started_at DESC
           LIMIT 200"#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| format!("DB list failed: {e}"))?;
    Ok(rows)
}

/// Delete a download record and optionally the file on disk.
pub async fn delete_download(
    db: &SqlitePool,
    download_id: &str,
    delete_file: bool,
) -> Result<(), String> {
    if delete_file {
        let path: Option<String> = sqlx::query_scalar!(
            "SELECT file_path FROM browser_downloads WHERE id = ?",
            download_id
        )
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .flatten();

        if let Some(p) = path {
            let _ = std::fs::remove_file(&p); // best-effort
        }
    }

    sqlx::query!("DELETE FROM browser_downloads WHERE id = ?", download_id)
        .execute(db)
        .await
        .map_err(|e| format!("DB delete failed: {e}"))?;
    Ok(())
}

/// Remove all downloads with status `imported`.
pub async fn clear_imported(db: &SqlitePool) -> Result<u64, String> {
    let result = sqlx::query!("DELETE FROM browser_downloads WHERE status = 'imported'")
        .execute(db)
        .await
        .map_err(|e| format!("DB clear_imported failed: {e}"))?;
    Ok(result.rows_affected())
}

/// Remove old downloads that exceed the retention period.
pub async fn clear_old_downloads(db: &SqlitePool) -> Result<u64, String> {
    let retention: i64 = sqlx::query_scalar!(
        "SELECT CAST(value AS INTEGER) FROM browser_settings WHERE key = 'retention_days'"
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(30);

    let interval = format!("-{retention}");
    let result = sqlx::query!(
        r#"DELETE FROM browser_downloads
           WHERE status IN ('finished', 'imported', 'failed', 'canceled')
             AND finished_at < datetime('now', ? || ' days')"#,
        interval
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB clear_old failed: {e}"))?;
    Ok(result.rows_affected())
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
) -> Result<(), String> {
    // Find the download by source_url + tab_label heuristic (most recent requested)
    use sqlx::Row;
    let row = sqlx::query(
        r#"SELECT id, session_id FROM browser_downloads
           WHERE source_url = ? AND status IN ('requested', 'in_progress')
           ORDER BY started_at DESC LIMIT 1"#,
    )
    .bind(source_url)
    .fetch_optional(db)
    .await
    .map_err(|e| format!("DB fetch failed: {e}"))?;

    let (download_id, session_id) = match row {
        Some(r) => (
            r.get::<String, _>("id"),
            r.get::<Option<String>, _>("session_id"),
        ),
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
        let auto_import: bool =
            sqlx::query_scalar!("SELECT value FROM browser_settings WHERE key = 'auto_import'")
                .fetch_optional(db)
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
