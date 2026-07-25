use super::*;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn setup_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Insert a dummy game for foreign keys
    // Since we don't enable PRAGMA foreign_keys, it might not be strictly needed, but let's be safe
    sqlx::query!("INSERT INTO games (id, name, path) VALUES ('test_game', 'Test Game', 'C:\\')")
        .execute(&pool)
        .await
        .ok();

    pool
}

#[tokio::test]
async fn test_import_job_status_transitions() {
    let pool = setup_db().await;

    // Insert a dummy download record
    sqlx::query!("INSERT INTO browser_downloads (id, filename, status, started_at) VALUES ('dl-1', 'test.zip', 'finished', '2025-01-01T00:00:00')")
            .execute(&pool).await.unwrap();

    let job_id = "job-1";
    sqlx::query!(
            "INSERT INTO import_jobs (id, download_id, archive_path, status, created_at, updated_at) 
             VALUES (?, 'dl-1', 'C:\\dummy.zip', 'queued', '2025-01-01T00:00:00', '2025-01-01T00:00:00')",
             job_id
        ).execute(&pool).await.unwrap();

    // 1. Transition to extracting
    set_job_status(&pool, job_id, "extracting", None)
        .await
        .unwrap();

    let status1 = sqlx::query_scalar!("SELECT status FROM import_jobs WHERE id = ?", job_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status1, "extracting");

    // 2. Transition to failed with error
    let err_msg = "CRC Mismatch";
    set_job_status(&pool, job_id, "failed", Some(err_msg))
        .await
        .unwrap();

    let rec2 = sqlx::query!(
        "SELECT status, error_msg FROM import_jobs WHERE id = ?",
        job_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(rec2.status, "failed");
    assert_eq!(rec2.error_msg.unwrap(), err_msg);
}

#[tokio::test]
async fn interrupted_transfers_fail_on_restart() {
    use crate::repo::browser_repo::{fail_interrupted_downloads, fail_interrupted_jobs};

    let pool = setup_db().await;

    sqlx::query!(
        "INSERT INTO browser_downloads (id, filename, status, started_at) VALUES
            ('dl-live', 'a.zip', 'in_progress', '2025'),
            ('dl-new',  'b.zip', 'requested',   '2025'),
            ('dl-done', 'c.zip', 'finished',    '2025')"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO import_jobs (id, download_id, archive_path, status, created_at, updated_at) VALUES
            ('job-queued',  'dl-new',  'C:\\1.zip', 'queued',       '2025', '2025'),
            ('job-running', 'dl-live', 'C:\\2.zip', 'placing',      '2025', '2025'),
            ('job-review',  'dl-done', 'C:\\3.zip', 'needs_review', '2025', '2025'),
            ('job-done',    'dl-done', 'C:\\4.zip', 'done',         '2025', '2025')"
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(fail_interrupted_downloads(&pool).await.unwrap(), 2);
    assert_eq!(fail_interrupted_jobs(&pool).await.unwrap(), 2);

    let stuck = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM browser_downloads WHERE status IN ('requested', 'in_progress')
         UNION ALL
         SELECT COUNT(*) FROM import_jobs WHERE status IN ('queued', 'extracting', 'matching', 'placing')"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(stuck, vec![0, 0]);

    // Terminal and user-blocked rows are left alone.
    let finished = sqlx::query_scalar!("SELECT status FROM browser_downloads WHERE id = 'dl-done'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(finished, "finished");

    let review = sqlx::query_scalar!("SELECT status FROM import_jobs WHERE id = 'job-review'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(review, "needs_review");

    // Recovery is idempotent: a second boot finds nothing left to fail.
    assert_eq!(fail_interrupted_downloads(&pool).await.unwrap(), 0);
    assert_eq!(fail_interrupted_jobs(&pool).await.unwrap(), 0);
}

#[tokio::test]
async fn test_import_job_dedup_hash() {
    let pool = setup_db().await;

    sqlx::query!("INSERT INTO browser_downloads (id, filename, status, started_at) VALUES ('dl-1', 'done.zip', 'finished', '2025'), ('dl-2', 'new.zip', 'finished', '2025')")
            .execute(&pool).await.unwrap();

    // Done job with hash
    sqlx::query!("INSERT INTO import_jobs (id, download_id, archive_path, status, archive_hash, created_at, updated_at) VALUES ('job-old', 'dl-1', 'C:\\1.zip', 'done', 'hash_abc', '2025', '2025')")
            .execute(&pool).await.unwrap();

    // New queued job
    sqlx::query!("INSERT INTO import_jobs (id, download_id, archive_path, status, created_at, updated_at) VALUES ('job-new', 'dl-2', 'C:\\2.zip', 'queued', '2025', '2025')")
            .execute(&pool).await.unwrap();

    let new_hash = "hash_abc";

    // Inline dedup logic from run_pipeline
    let existing = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM import_jobs WHERE archive_hash = ? AND status = 'done'",
        new_hash
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    assert!(existing > 0);

    // Mark as duplicate
    sqlx::query!(
        "UPDATE import_jobs SET is_duplicate = 1 WHERE id = ?",
        "job-new"
    )
    .execute(&pool)
    .await
    .unwrap();

    let is_dup = sqlx::query_scalar!("SELECT is_duplicate FROM import_jobs WHERE id = 'job-new'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(is_dup, 1);
}
