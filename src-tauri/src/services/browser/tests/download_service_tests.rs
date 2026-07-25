//! Characterization tests for `download_service` — they pin the behaviour that
//! exists today (including the quirks), not an idealized contract.

use super::*;
use crate::repo::browser_repo;
use crate::test_utils::init_test_db;

async fn status_of(db: &SqlitePool, id: &str) -> Option<String> {
    list_downloads(db)
        .await
        .unwrap()
        .into_iter()
        .find(|d| d.id == id)
        .map(|d| d.status)
}

#[tokio::test]
async fn create_download_lands_as_requested_row_in_the_listing() {
    let db = init_test_db().await.pool;

    let id = create_download(
        &db,
        Some("sess-1"),
        "pack.zip",
        "https://x/pack.zip",
        "C:/dl/pack.zip",
    )
    .await
    .unwrap();

    let rows = list_downloads(&db).await.unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.id, id);
    assert_eq!(row.status, "requested");
    assert_eq!(row.bytes_received, 0);
    assert_eq!(row.session_id.as_deref(), Some("sess-1"));
    assert_eq!(row.file_path.as_deref(), Some("C:/dl/pack.zip"));
    assert!(row.finished_at.is_none());
}

#[tokio::test]
async fn update_status_stamps_finished_at_only_for_terminal_states() {
    let db = init_test_db().await.pool;
    let id = create_download(&db, None, "a.zip", "https://x/a.zip", "C:/dl/a.zip")
        .await
        .unwrap();

    update_status(&db, &id, "in_progress", Some(10), Some(100), None, None)
        .await
        .unwrap();
    let row = list_downloads(&db).await.unwrap().remove(0);
    assert_eq!(row.status, "in_progress");
    assert_eq!(row.bytes_received, 10);
    assert_eq!(row.bytes_total, Some(100));
    assert!(row.finished_at.is_none());

    update_status(&db, &id, "finished", None, None, None, None)
        .await
        .unwrap();
    let row = list_downloads(&db).await.unwrap().remove(0);
    assert_eq!(row.status, "finished");
    assert!(row.finished_at.is_some());
    // COALESCE keeps the earlier progress values.
    assert_eq!(row.bytes_received, 10);
}

#[tokio::test]
async fn cancel_download_marks_stale_record_canceled_and_keeps_the_row() {
    let db = init_test_db().await.pool;
    let id = create_download(&db, None, "b.zip", "https://x/b.zip", "C:/dl/b.zip")
        .await
        .unwrap();

    // Nothing is in flight, so the DB path runs.
    cancel_download(&db, &id, None).await.unwrap();

    assert_eq!(status_of(&db, &id).await.as_deref(), Some("canceled"));
}

#[tokio::test]
async fn cancel_download_with_delete_file_drops_row_and_file() {
    let db = init_test_db().await.pool;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("c.zip");
    std::fs::write(&file, b"payload").unwrap();

    let id = create_download(
        &db,
        None,
        "c.zip",
        "https://x/c.zip",
        &file.to_string_lossy(),
    )
    .await
    .unwrap();

    cancel_download(&db, &id, Some(true)).await.unwrap();

    assert!(!file.exists());
    assert!(list_downloads(&db).await.unwrap().is_empty());
}

#[tokio::test]
async fn clear_imported_removes_only_imported_rows() {
    let db = init_test_db().await.pool;
    let keep = create_download(&db, None, "k.zip", "https://x/k.zip", "C:/dl/k.zip")
        .await
        .unwrap();
    let drop = create_download(&db, None, "d.zip", "https://x/d.zip", "C:/dl/d.zip")
        .await
        .unwrap();
    update_status(&db, &drop, "imported", None, None, None, None)
        .await
        .unwrap();

    let removed = clear_imported(&db).await.unwrap();

    assert_eq!(removed, 1);
    assert_eq!(status_of(&db, &keep).await.as_deref(), Some("requested"));
    assert!(status_of(&db, &drop).await.is_none());
}

#[tokio::test]
async fn clear_old_downloads_uses_the_configured_retention_window() {
    let db = init_test_db().await.pool;
    browser_repo::set_setting(&db, "retention_days", "1")
        .await
        .unwrap();

    let stale = create_download(&db, None, "old.zip", "https://x/old.zip", "C:/dl/old.zip")
        .await
        .unwrap();
    let fresh = create_download(&db, None, "new.zip", "https://x/new.zip", "C:/dl/new.zip")
        .await
        .unwrap();

    // `update_status` always stamps "now", so backdate the stale row directly.
    update_status(&db, &stale, "finished", None, None, None, None)
        .await
        .unwrap();
    update_status(&db, &fresh, "finished", None, None, None, None)
        .await
        .unwrap();
    sqlx::query("UPDATE browser_downloads SET finished_at = '2020-01-01T00:00:00' WHERE id = ?")
        .bind(&stale)
        .execute(&db)
        .await
        .unwrap();

    let removed = clear_old_downloads(&db).await.unwrap();

    assert_eq!(removed, 1);
    assert!(status_of(&db, &stale).await.is_none());
    assert_eq!(status_of(&db, &fresh).await.as_deref(), Some("finished"));
}
