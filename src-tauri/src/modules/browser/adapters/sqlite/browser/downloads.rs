//! `browser_downloads` persistence.

use crate::modules::browser::domain::browser::BrowserDownloadDto;
use sqlx::{Row, SqlitePool};

/// Row of a still-open download matched by source URL.
pub struct ActiveDownloadRow {
    pub id: String,
    pub session_id: Option<String>,
}

/// Row of a finished download that is eligible for import.
pub struct ImportableDownloadRow {
    pub file_path: Option<String>,
    pub session_id: Option<String>,
}

/// Terminal download metadata retained for an explicit retry request.
pub struct RetryableDownloadRow {
    pub session_id: Option<String>,
    pub filename: String,
    pub source_url: Option<String>,
}

/// Insert a new `requested` download record.
pub struct NewDownloadRow<'a> {
    pub id: &'a str,
    pub session_id: Option<&'a str>,
    pub filename: &'a str,
    pub source_url: &'a str,
    pub file_path: &'a str,
    pub queue_order: i64,
    pub started_at: &'a str,
}

pub async fn insert_download(
    db: &SqlitePool,
    download: NewDownloadRow<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO browser_downloads
           (id, session_id, filename, file_path, source_url, status, bytes_received, queue_order, started_at)
           VALUES (?, ?, ?, ?, ?, 'requested', 0, ?, ?)"#,
    )
    .bind(download.id)
    .bind(download.session_id)
    .bind(download.filename)
    .bind(download.file_path)
    .bind(download.source_url)
    .bind(download.queue_order)
    .bind(download.started_at)
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
    .await?;
    Ok(())
}

/// Fail every transfer left in flight: the reqwest stream dies with the process
/// and nothing resumes it, so those rows would otherwise sit in `in_progress`
/// forever. Returns the number of rows recovered.
pub async fn fail_interrupted_downloads(db: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE browser_downloads
            SET status = 'failed',
                error_msg = COALESCE(error_msg, 'Interrupted by app restart'),
                finished_at = COALESCE(finished_at, datetime('now'))
          WHERE status IN ('requested', 'in_progress')"
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// List the 200 most recent downloads.
pub async fn list_downloads(db: &SqlitePool) -> Result<Vec<BrowserDownloadDto>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, filename, file_path, source_url,
                  status, bytes_total, bytes_received, error_msg, queue_order,
                  started_at, finished_at
           FROM browser_downloads
           ORDER BY started_at DESC, queue_order DESC
           LIMIT 200"#,
    )
    .fetch_all(db)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(BrowserDownloadDto {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                filename: row.try_get("filename")?,
                file_path: row.try_get("file_path")?,
                source_url: row.try_get("source_url")?,
                status: row.try_get("status")?,
                bytes_total: row.try_get("bytes_total")?,
                bytes_received: row.try_get("bytes_received")?,
                error_msg: row.try_get("error_msg")?,
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

/// Delete every download whose status is `imported`. Returns rows removed.
pub async fn delete_imported(db: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM browser_downloads WHERE status = 'imported'")
        .execute(db)
        .await?;
    Ok(result.rows_affected())
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

/// Most recent still-open download for a source URL.
pub async fn find_active_by_url(
    db: &SqlitePool,
    source_url: &str,
) -> Result<Option<ActiveDownloadRow>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        r#"SELECT id, session_id FROM browser_downloads
           WHERE source_url = ? AND status IN ('requested', 'in_progress')
           ORDER BY started_at DESC LIMIT 1"#,
    )
    .bind(source_url)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| ActiveDownloadRow {
        id: r.get::<String, _>("id"),
        session_id: r.get::<Option<String>, _>("session_id"),
    }))
}

/// Read a canceled or failed download that may be explicitly queued again.
pub async fn get_retryable_download(
    db: &SqlitePool,
    download_id: &str,
) -> Result<Option<RetryableDownloadRow>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT session_id, filename, source_url
           FROM browser_downloads
          WHERE id = ? AND status IN ('failed', 'canceled')",
    )
    .bind(download_id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|row| RetryableDownloadRow {
        session_id: row.get("session_id"),
        filename: row.get("filename"),
        source_url: row.get("source_url"),
    }))
}

/// Fetch a `finished` download so it can be queued for import.
pub async fn get_finished_for_import(
    db: &SqlitePool,
    download_id: &str,
) -> Result<Option<ImportableDownloadRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT file_path, session_id FROM browser_downloads WHERE id = ? AND status = 'finished'",
        download_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| ImportableDownloadRow {
        file_path: r.file_path,
        session_id: r.session_id,
    }))
}

/// Flag a download as `imported` (runs inside the placement transaction).
pub async fn mark_imported(
    conn: &mut sqlx::SqliteConnection,
    download_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE browser_downloads SET status = 'imported' WHERE id = ?",
        download_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
