//! `import_jobs` persistence (plus the `download_sessions` lookup the queue needs).
//!
//! NOTE: `match_entry_key` / `match_alias_name` are newer columns than the
//! checked-in `app.db`, so every statement touching them stays on the runtime
//! `sqlx::query` API instead of the compile-time macros.

use crate::domain::browser::ImportJobDto;
use sqlx::SqlitePool;

/// The deep-match outcome stored on an import job.
pub struct ImportJobMatch {
    pub category: Option<String>,
    pub entry_key: Option<String>,
    pub alias_name: Option<String>,
    pub confidence: f64,
    pub reason: Option<String>,
}

/// Game configured on the download session an archive came from.
pub async fn get_session_game_id(
    db: &SqlitePool,
    session_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let game_id = sqlx::query_scalar!(
        "SELECT game_id FROM download_sessions WHERE id = ?",
        session_id
    )
    .fetch_optional(db)
    .await?;
    Ok(game_id.flatten())
}

/// Insert a fresh `queued` job.
pub async fn insert_job(
    db: &SqlitePool,
    job_id: &str,
    download_id: &str,
    game_id: Option<&str>,
    archive_path: &str,
    now: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO import_jobs
           (id, download_id, game_id, archive_path, status, is_duplicate, created_at, updated_at)
           VALUES (?, ?, ?, ?, 'queued', 0, ?, ?)"#,
        job_id,
        download_id,
        game_id,
        archive_path,
        now,
        now
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Count already-completed jobs carrying the same archive hash.
pub async fn count_done_with_hash(db: &SqlitePool, hash: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT COUNT(*) FROM import_jobs WHERE archive_hash = ? AND status = 'done'",
        hash
    )
    .fetch_one(db)
    .await
}

/// Flag a job as a duplicate of an already-imported archive.
pub async fn mark_duplicate(db: &SqlitePool, job_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE import_jobs SET is_duplicate = 1 WHERE id = ?",
        job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Persist the BLAKE3 hash of the source archive.
pub async fn set_archive_hash(
    db: &SqlitePool,
    job_id: &str,
    hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE import_jobs SET archive_hash = ? WHERE id = ?",
        hash,
        job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Persist the staging copy location.
pub async fn set_staging_path(
    db: &SqlitePool,
    job_id: &str,
    staging_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE import_jobs SET staging_path = ? WHERE id = ?",
        staging_path,
        job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Game selected for a job (set at queue time or by manual review).
pub async fn get_job_game_id(db: &SqlitePool, job_id: &str) -> Result<Option<String>, sqlx::Error> {
    let game_id = sqlx::query_scalar!("SELECT game_id FROM import_jobs WHERE id = ?", job_id)
        .fetch_optional(db)
        .await?;
    Ok(game_id.flatten())
}

/// Store the deep-match outcome on the job row.
pub async fn set_match_result(
    db: &SqlitePool,
    job_id: &str,
    matched: &ImportJobMatch,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs
             SET match_category = ?, match_entry_key = ?, match_alias_name = ?, match_confidence = ?, match_reason = ?
             WHERE id = ?",
    )
    .bind(&matched.category)
    .bind(&matched.entry_key)
    .bind(&matched.alias_name)
    .bind(matched.confidence)
    .bind(&matched.reason)
    .bind(job_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Read back the stored deep-match outcome.
pub async fn load_match_result(
    db: &SqlitePool,
    job_id: &str,
) -> Result<ImportJobMatch, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        "SELECT match_category, match_entry_key, match_alias_name, match_confidence, match_reason
         FROM import_jobs
         WHERE id = ?",
    )
    .bind(job_id)
    .fetch_one(db)
    .await?;

    Ok(ImportJobMatch {
        category: row.try_get("match_category").unwrap_or(None),
        entry_key: row.try_get("match_entry_key").unwrap_or(None),
        alias_name: row.try_get("match_alias_name").unwrap_or(None),
        confidence: row
            .try_get("match_confidence")
            .unwrap_or(Some(0.0))
            .unwrap_or(0.0),
        reason: row.try_get("match_reason").unwrap_or(None),
    })
}

/// Move a job to a new status, optionally recording an error message.
pub async fn set_status(
    db: &SqlitePool,
    job_id: &str,
    status: &str,
    error_msg: Option<&str>,
    now: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE import_jobs SET status = ?, error_msg = COALESCE(?, error_msg), updated_at = ? WHERE id = ?",
        status, error_msg, now, job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Fail every job left mid-pipeline: nothing resumes a run after the process
/// dies, so those rows would otherwise sit in `extracting`/`placing` forever.
/// `needs_review` is untouched — it is waiting on the user, not on a worker.
/// Returns the number of rows recovered.
pub async fn fail_interrupted_jobs(db: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE import_jobs
            SET status = 'failed',
                error_msg = COALESCE(error_msg, 'Interrupted by app restart'),
                updated_at = datetime('now')
          WHERE status IN ('queued', 'extracting', 'matching', 'placing')"
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// The 100 most recent non-canceled jobs.
pub async fn list_active_jobs(db: &SqlitePool) -> Result<Vec<ImportJobDto>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"SELECT id, download_id, game_id, archive_path, status,
                  match_category, match_entry_key, match_alias_name, match_confidence, match_reason,
                  placed_path, error_msg, is_duplicate, created_at, updated_at
           FROM import_jobs
           WHERE status != 'canceled'
           ORDER BY created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ImportJobDto {
            id: r.try_get("id").unwrap_or_default(),
            download_id: r.try_get("download_id").unwrap_or(None),
            game_id: r.try_get("game_id").unwrap_or(None),
            archive_path: r.try_get("archive_path").unwrap_or_default(),
            status: r.try_get("status").unwrap_or_default(),
            match_category: r.try_get("match_category").unwrap_or(None),
            match_entry_key: r.try_get("match_entry_key").unwrap_or(None),
            match_alias_name: r.try_get("match_alias_name").unwrap_or(None),
            match_confidence: r.try_get("match_confidence").unwrap_or(None),
            match_reason: r.try_get("match_reason").unwrap_or(None),
            placed_path: r.try_get("placed_path").unwrap_or(None),
            error_msg: r.try_get("error_msg").unwrap_or(None),
            is_duplicate: r.try_get::<i64, _>("is_duplicate").unwrap_or(0) != 0,
            created_at: r.try_get("created_at").unwrap_or_default(),
            updated_at: r.try_get("updated_at").unwrap_or_default(),
        })
        .collect())
}

/// Apply the operator's manual review decision and hand the job to placement.
pub async fn apply_review_decision(
    db: &SqlitePool,
    job_id: &str,
    game_id: &str,
    category: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs
         SET game_id = ?, match_category = ?, status = 'placing', updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(game_id)
    .bind(category)
    .bind(job_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Staging path of a job that must exist — errors when the job row is gone.
pub async fn require_staging_path(
    db: &SqlitePool,
    job_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!("SELECT staging_path FROM import_jobs WHERE id = ?", job_id)
        .fetch_one(db)
        .await
}

/// Staging path of a job that may be gone already.
pub async fn get_staging_path(
    db: &SqlitePool,
    job_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let staging = sqlx::query_scalar!("SELECT staging_path FROM import_jobs WHERE id = ?", job_id)
        .fetch_optional(db)
        .await?;
    Ok(staging.flatten())
}

/// Mark a job `canceled`.
pub async fn mark_canceled(db: &SqlitePool, job_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE import_jobs SET status = 'canceled', updated_at = datetime('now') WHERE id = ?",
        job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Record the final on-disk location and close the job (inside the placement tx).
pub async fn set_placed_done(
    conn: &mut sqlx::SqliteConnection,
    job_id: &str,
    placed_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE import_jobs SET placed_path = ?, status = 'done', updated_at = datetime('now') WHERE id = ?",
        placed_path, job_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Download a job originated from (inside the placement tx).
pub async fn get_download_id(
    conn: &mut sqlx::SqliteConnection,
    job_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let download_id =
        sqlx::query_scalar!("SELECT download_id FROM import_jobs WHERE id = ?", job_id)
            .fetch_optional(conn)
            .await?;
    Ok(download_id.flatten())
}
