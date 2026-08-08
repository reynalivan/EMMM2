//! `browser_downloads` persistence.

use crate::domain::browser::BrowserDownloadDto;
use sqlx::SqlitePool;

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

/// Insert a new `requested` download record.
pub async fn insert_download(
    db: &SqlitePool,
    id: &str,
    session_id: Option<&str>,
    filename: &str,
    source_url: &str,
    file_path: &str,
    started_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO browser_downloads
           (id, session_id, filename, file_path, source_url, status, bytes_received, started_at)
           VALUES (?, ?, ?, ?, ?, 'requested', 0, ?)"#,
        id,
        session_id,
        filename,
        file_path,
        source_url,
        started_at
    )
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
    sqlx::query_as::<_, BrowserDownloadDto>(
        r#"SELECT id, session_id, filename, file_path, source_url,
                  status, bytes_total, bytes_received, error_msg,
                  started_at, finished_at
           FROM browser_downloads
           ORDER BY started_at DESC
           LIMIT 200"#,
    )
    .fetch_all(db)
    .await
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
