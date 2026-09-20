//! `browser_downloads` persistence.

use crate::modules::browser::domain::browser::{BrowserDownloadDto, BrowserGameBananaProvenance};
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
    pub origin_page_url: Option<&'a str>,
    pub gamebanana_item_type: Option<&'a str>,
    pub gamebanana_item_id: Option<u64>,
}

pub async fn insert_download(
    db: &SqlitePool,
    download: NewDownloadRow<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO browser_downloads
           (id, game_id, session_id, filename, file_path, source_url, status, bytes_received, can_resume, queue_order, started_at, tab_label,
            origin_page_url, gamebanana_item_type, gamebanana_item_id)
           VALUES (?, ?, ?, ?, ?, ?, 'requested', 0, 0, ?, ?, ?, ?, ?, ?)"#,
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
    .bind(download.origin_page_url)
    .bind(download.gamebanana_item_type)
    .bind(download.gamebanana_item_id.map(|value| value as i64))
    .execute(db)
    .await?;
    Ok(())
}

/// Return provenance only for a finished Discover source whose current size and
/// content hash still match the completed browser download.
pub async fn find_gamebanana_provenance_for_source_path(
    db: &SqlitePool,
    game_id: &str,
    source_path: &str,
    source_size_bytes: i64,
    source_sha256: &str,
) -> Result<Option<BrowserGameBananaProvenance>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT origin_page_url, gamebanana_item_type, gamebanana_item_id
           FROM browser_downloads
          WHERE game_id = ?
            AND file_path = ? COLLATE NOCASE
            AND origin_page_url IS NOT NULL
            AND gamebanana_item_type IS NOT NULL
            AND gamebanana_item_id IS NOT NULL
            AND status = 'finished'
            AND bytes_received = ?
            AND gamebanana_content_sha256 = ?
          ORDER BY started_at DESC, queue_order DESC
          LIMIT 1",
    )
    .bind(game_id)
    .bind(source_path)
    .bind(source_size_bytes)
    .bind(source_sha256)
    .fetch_optional(db)
    .await?;

    row.map(|row| {
        let item_id = row.try_get::<i64, _>("gamebanana_item_id")?;
        Ok(BrowserGameBananaProvenance {
            origin_page_url: row.try_get("origin_page_url")?,
            item_type: row.try_get("gamebanana_item_type")?,
            item_id: u64::try_from(item_id).map_err(|_| {
                sqlx::Error::Decode("negative GameBanana item id in browser download".into())
            })?,
        })
    })
    .transpose()
}

/// Save the completed content hash only for a verified GameBanana download.
pub async fn store_gamebanana_content_hash(
    db: &SqlitePool,
    download_id: &str,
    content_sha256: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE browser_downloads
            SET gamebanana_content_sha256 = ?
          WHERE id = ?
            AND status = 'finished'
            AND origin_page_url IS NOT NULL
            AND gamebanana_item_type IS NOT NULL
            AND gamebanana_item_id IS NOT NULL",
    )
    .bind(content_sha256)
    .bind(download_id)
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
          WHERE status IN ('requested', 'in_progress', 'paused')",
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

#[cfg(test)]
mod tests {
    use super::{
        find_gamebanana_provenance_for_source_path, insert_download, store_gamebanana_content_hash,
        update_status, NewDownloadRow,
    };

    #[tokio::test]
    async fn provenance_requires_a_finished_download_with_matching_signature() {
        let context = crate::test_utils::init_test_db().await;
        let source_path = "C:/Games/Genshin/Mods/.mod-inbox/ayaka.zip";
        insert_download(
            &context.pool,
            NewDownloadRow {
                id: "gamebanana-download",
                game_id: "gimi",
                session_id: None,
                filename: "ayaka.zip",
                source_url: "https://cdn.gamebanana.com/ayaka.zip",
                file_path: source_path,
                queue_order: 1,
                started_at: "2026-09-14T00:00:00",
                tab_label: None,
                origin_page_url: Some("https://gamebanana.com/mods/528562"),
                gamebanana_item_type: Some("Mod"),
                gamebanana_item_id: Some(528562),
            },
        )
        .await
        .expect("insert browser download");

        assert!(find_gamebanana_provenance_for_source_path(
            &context.pool,
            "gimi",
            source_path,
            2048,
            "download-sha",
        )
        .await
        .expect("look up requested download")
        .is_none());

        update_status(
            &context.pool,
            "gamebanana-download",
            "finished",
            Some(2048),
            Some(2048),
            None,
            Some(source_path),
            Some("2026-09-14T00:00:01".to_string()),
        )
        .await
        .expect("finish browser download");
        store_gamebanana_content_hash(&context.pool, "gamebanana-download", "download-sha")
            .await
            .expect("store browser download hash");

        assert!(find_gamebanana_provenance_for_source_path(
            &context.pool,
            "gimi",
            source_path,
            1024,
            "download-sha",
        )
        .await
        .expect("look up mismatched file")
        .is_none());
        assert!(find_gamebanana_provenance_for_source_path(
            &context.pool,
            "gimi",
            source_path,
            2048,
            "wrong-sha",
        )
        .await
        .expect("look up mismatched content")
        .is_none());
        assert_eq!(
            find_gamebanana_provenance_for_source_path(
                &context.pool,
                "gimi",
                source_path,
                2048,
                "download-sha",
            )
            .await
            .expect("look up finished download")
            .map(|provenance| provenance.item_id),
            Some(528562)
        );
    }
}
