//! `browser_downloads` persistence.

use crate::modules::browser::domain::browser::BrowserDownloadDto;
use sqlx::{Row, SqlitePool};

/// Terminal download metadata retained for an explicit retry request.
pub struct RetryableDownloadRow {
    pub game_id: String,
    pub session_id: Option<String>,
    pub filename: String,
    pub source_url: Option<String>,
    pub tab_label: Option<String>,
}

/// Persist a WebView2 operation state. Unlike the fallback worker, an
/// interrupted native operation may remain resumable while its WebView lives.
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
    finished_at: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE browser_downloads SET
            status         = ?,
            bytes_received = COALESCE(?, bytes_received),
            bytes_total    = COALESCE(?, bytes_total),
            error_msg      = ?,
            can_resume     = ?,
            file_path      = COALESCE(?, file_path),
            finished_at    = COALESCE(?, finished_at)
          WHERE id = ?"#,
    )
    .bind(status)
    .bind(bytes_received)
    .bind(bytes_total)
    .bind(error_msg)
    .bind(i64::from(can_resume))
    .bind(file_path)
    .bind(finished_at)
    .bind(download_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Insert a new `requested` download record.
pub struct NewDownloadRow<'a> {
    pub id: &'a str,
    pub game_id: &'a str,
    pub session_id: Option<&'a str>,
    pub filename: &'a str,
    pub source_url: &'a str,
    pub file_path: &'a str,
    pub queue_order: i64,
    pub started_at: &'a str,
    pub tab_label: Option<&'a str>,
}

pub async fn insert_download(
    db: &SqlitePool,
    download: NewDownloadRow<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO browser_downloads
           (id, game_id, session_id, filename, file_path, source_url, status, bytes_received, can_resume, queue_order, started_at, tab_label)
           VALUES (?, ?, ?, ?, ?, ?, 'requested', 0, 0, ?, ?, ?)"#,
    )
    .bind(download.id)
    .bind(download.game_id)
    .bind(download.session_id)
    .bind(download.filename)
    .bind(download.file_path)
    .bind(download.source_url)
    .bind(download.queue_order)
    .bind(download.started_at)
    .bind(download.tab_label)
    .execute(db)
    .await?;
    Ok(())
}

/// Update status plus optional progress columns. `finished_at` is only applied
/// when `Some` (the caller decides which statuses are terminal).
#[allow(clippy::too_many_arguments)]
pub async fn update_status(
    db: &SqlitePool,
    download_id: &str,
    status: &str,
    bytes_received: Option<i64>,
    bytes_total: Option<i64>,
    error_msg: Option<&str>,
    file_path: Option<&str>,
    finished_at: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE browser_downloads SET
            status         = ?,
            bytes_received = COALESCE(?, bytes_received),
            bytes_total    = COALESCE(?, bytes_total),
            error_msg      = ?,
            can_resume     = 0,
            file_path      = COALESCE(?, file_path),
            finished_at    = COALESCE(?, finished_at)
          WHERE id = ?"#,
    )
    .bind(status)
    .bind(bytes_received)
    .bind(bytes_total)
    .bind(error_msg)
    .bind(file_path)
    .bind(finished_at)
    .bind(download_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Fail every transfer left in flight: the reqwest stream dies with the process
/// and nothing resumes it, so those rows would otherwise sit in `in_progress`
/// forever. Returns the number of rows recovered.
pub async fn fail_interrupted_downloads(db: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE browser_downloads
            SET status = 'failed',
                error_msg = COALESCE(error_msg, 'Interrupted by app restart'),
                can_resume = 0,
                finished_at = COALESCE(finished_at, datetime('now'))
          WHERE status IN ('requested', 'in_progress', 'paused')"
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// List the 200 most recent downloads.
pub async fn list_downloads(
    db: &SqlitePool,
    game_id: &str,
) -> Result<Vec<BrowserDownloadDto>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT id, game_id, session_id, filename, file_path, source_url,
                  status, bytes_total, bytes_received, error_msg, can_resume, tab_label, queue_order,
                  started_at, finished_at
           FROM browser_downloads WHERE game_id = ?
           ORDER BY started_at DESC, queue_order DESC
           LIMIT 200"#,
    )
    .bind(game_id)
    .fetch_all(db)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(BrowserDownloadDto {
                id: row.try_get("id")?,
                game_id: row.try_get("game_id")?,
                session_id: row.try_get("session_id")?,
                filename: row.try_get("filename")?,
                file_path: row.try_get("file_path")?,
                source_url: row.try_get("source_url")?,
                status: row.try_get("status")?,
                bytes_total: row.try_get("bytes_total")?,
                bytes_received: row.try_get("bytes_received")?,
                error_msg: row.try_get("error_msg")?,
                can_resume: Some(row.try_get("can_resume")?),
                tab_label: row.try_get("tab_label")?,
                queue_order: row.try_get("queue_order")?,
                started_at: row.try_get("started_at")?,
                finished_at: row.try_get("finished_at")?,
            })
        })
        .collect()
}

/// Read the stored file path of a download.
pub async fn get_file_path(
    db: &SqlitePool,
    download_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let path = sqlx::query_scalar!(
        "SELECT file_path FROM browser_downloads WHERE id = ?",
        download_id
    )
    .fetch_optional(db)
    .await?;
    Ok(path.flatten())
}

/// Delete a single download record.
pub async fn delete_download(db: &SqlitePool, download_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM browser_downloads WHERE id = ?", download_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Delete terminal downloads finished more than `retention_days` ago.
pub async fn delete_older_than(db: &SqlitePool, retention_days: i64) -> Result<u64, sqlx::Error> {
    let interval = format!("-{retention_days}");
    let result = sqlx::query!(
        r#"DELETE FROM browser_downloads
           WHERE status IN ('finished', 'imported', 'failed', 'canceled')
             AND finished_at < datetime('now', ? || ' days')"#,
        interval
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// Read a canceled or failed download that may be explicitly queued again.
pub async fn get_retryable_download(
    db: &SqlitePool,
    download_id: &str,
) -> Result<Option<RetryableDownloadRow>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT game_id, session_id, filename, source_url, tab_label
           FROM browser_downloads
          WHERE id = ? AND status IN ('failed', 'canceled')",
    )
    .bind(download_id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|row| RetryableDownloadRow {
        game_id: row.get("game_id"),
        session_id: row.get("session_id"),
        filename: row.get("filename"),
        source_url: row.get("source_url"),
        tab_label: row.get("tab_label"),
    }))
}
