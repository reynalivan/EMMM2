//! Persisted Discover metadata. This deliberately stores only display and
//! navigation metadata; WebView2 owns cookies and authentication state.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::shared::errors::BrowserError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, sqlx::FromRow)]
pub struct BrowserBookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, sqlx::FromRow)]
pub struct BrowserHistoryEntry {
    pub url: String,
    pub hostname: String,
    pub title: String,
    pub favicon: Option<String>,
    pub visit_count: i64,
    pub last_visited_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BrowserSessionTab {
    pub position: i64,
    pub url: String,
    pub title: String,
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BrowserPrivacySummary {
    pub bookmarks: i64,
    pub history_entries: i64,
    pub saved_permissions: i64,
}

pub async fn list_bookmarks(db: &SqlitePool) -> Result<Vec<BrowserBookmark>, BrowserError> {
    Ok(sqlx::query_as::<_, BrowserBookmark>(
        "SELECT id, url, title, favicon, created_at, updated_at FROM browser_bookmarks ORDER BY updated_at DESC",
    )
    .fetch_all(db)
    .await?)
}

pub async fn add_bookmark(
    db: &SqlitePool,
    url: &str,
    title: Option<&str>,
    favicon: Option<&str>,
) -> Result<BrowserBookmark, BrowserError> {
    let url = canonical_http_url(url)?;
    let now = Utc::now().timestamp();
    let id = uuid::Uuid::new_v4().to_string();
    let title = title.unwrap_or_default().trim();
    let favicon = favicon.filter(|value| value.starts_with("data:image/"));
    sqlx::query(
        "INSERT INTO browser_bookmarks (id, url, title, favicon, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(url) DO UPDATE SET title = excluded.title, favicon = excluded.favicon, updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(&url)
    .bind(title)
    .bind(favicon)
    .bind(now)
    .bind(now)
    .execute(db)
    .await?;
    sqlx::query_as::<_, BrowserBookmark>(
        "SELECT id, url, title, favicon, created_at, updated_at FROM browser_bookmarks WHERE url = ?",
    )
    .bind(url)
    .fetch_one(db)
    .await
    .map_err(Into::into)
}

pub async fn delete_bookmark(db: &SqlitePool, id: &str) -> Result<(), BrowserError> {
    sqlx::query("DELETE FROM browser_bookmarks WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn update_bookmark(
    db: &SqlitePool,
    id: &str,
    url: &str,
    title: &str,
) -> Result<BrowserBookmark, BrowserError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(BrowserError::InvalidSetting(
            "bookmark id must not be empty".to_string(),
        ));
    }

    let url = canonical_http_url(url)?;
    let duplicate_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM browser_bookmarks WHERE url = ? AND id != ?",
    )
    .bind(&url)
    .bind(id)
    .fetch_optional(db)
    .await?;
    if duplicate_id.is_some() {
        return Err(BrowserError::InvalidSetting(
            "another bookmark already uses this URL".to_string(),
        ));
    }

    let update =
        sqlx::query("UPDATE browser_bookmarks SET url = ?, title = ?, updated_at = ? WHERE id = ?")
            .bind(&url)
            .bind(title.trim())
            .bind(Utc::now().timestamp())
            .bind(id)
            .execute(db)
            .await?;
    if update.rows_affected() == 0 {
        return Err(BrowserError::InvalidSetting(
            "bookmark no longer exists".to_string(),
        ));
    }

    sqlx::query_as::<_, BrowserBookmark>(
        "SELECT id, url, title, favicon, created_at, updated_at FROM browser_bookmarks WHERE id = ?",
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(Into::into)
}

pub async fn record_history(
    db: &SqlitePool,
    url: &str,
    title: Option<&str>,
    favicon: Option<&str>,
) -> Result<(), BrowserError> {
    let url = canonical_http_url(url)?;
    let hostname = tauri::Url::parse(&url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
        .unwrap_or_default();
    let now = Utc::now().timestamp();
    let favicon = favicon.filter(|value| value.starts_with("data:image/"));
    sqlx::query(
        "INSERT INTO browser_history (url, hostname, title, favicon, visit_count, last_visited_at)
         VALUES (?, ?, ?, ?, 1, ?)
         ON CONFLICT(url) DO UPDATE SET hostname = excluded.hostname, title = excluded.title,
         favicon = COALESCE(excluded.favicon, browser_history.favicon),
         visit_count = browser_history.visit_count + 1, last_visited_at = excluded.last_visited_at",
    )
    .bind(url)
    .bind(hostname)
    .bind(title.unwrap_or_default().trim())
    .bind(favicon)
    .bind(now)
    .execute(db)
    .await?;
    sqlx::query(
        "DELETE FROM browser_history WHERE url IN (
            SELECT url FROM browser_history ORDER BY last_visited_at DESC LIMIT -1 OFFSET 1000
        )",
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Refresh display metadata without counting another visit.
pub async fn update_history_metadata(
    db: &SqlitePool,
    url: &str,
    title: &str,
) -> Result<(), BrowserError> {
    let url = canonical_http_url(url)?;
    sqlx::query("UPDATE browser_history SET title = ? WHERE url = ?")
        .bind(title.trim())
        .bind(url)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn list_history(
    db: &SqlitePool,
    limit: i64,
) -> Result<Vec<BrowserHistoryEntry>, BrowserError> {
    let limit = limit.clamp(1, 500);
    Ok(sqlx::query_as::<_, BrowserHistoryEntry>(
        "SELECT url, hostname, title, favicon, visit_count, last_visited_at
         FROM browser_history ORDER BY last_visited_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(db)
    .await?)
}

pub async fn clear_history(db: &SqlitePool) -> Result<(), BrowserError> {
    sqlx::query("DELETE FROM browser_history")
        .execute(db)
        .await?;
    Ok(())
}

pub async fn save_session_tabs(
    db: &SqlitePool,
    game_id: &str,
    tabs: &[BrowserSessionTab],
) -> Result<(), BrowserError> {
    if tabs.len() > 12 {
        return Err(BrowserError::InvalidSetting(
            "Discover session supports at most 12 tabs".into(),
        ));
    }
    let mut transaction = db.begin().await?;
    sqlx::query("DELETE FROM browser_session_tabs WHERE game_id = ?")
        .bind(game_id)
        .execute(&mut *transaction)
        .await?;
    let now = Utc::now().timestamp();
    for tab in tabs {
        let url = canonical_http_url(&tab.url)?;
        sqlx::query(
            "INSERT INTO browser_session_tabs (game_id, position, url, title, active, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(game_id)
        .bind(tab.position)
        .bind(url)
        .bind(tab.title.trim())
        .bind(tab.active)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn get_session_tabs(
    db: &SqlitePool,
    game_id: &str,
) -> Result<Vec<BrowserSessionTab>, BrowserError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        position: i64,
        url: String,
        title: String,
        active: i64,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT position, url, title, active FROM browser_session_tabs WHERE game_id = ? ORDER BY position ASC",
    )
    .bind(game_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| BrowserSessionTab {
            position: row.position,
            url: row.url,
            title: row.title,
            active: row.active != 0,
        })
        .collect())
}

pub async fn privacy_summary(db: &SqlitePool) -> Result<BrowserPrivacySummary, BrowserError> {
    let bookmarks = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM browser_bookmarks")
        .fetch_one(db)
        .await?;
    let history_entries = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM browser_history")
        .fetch_one(db)
        .await?;
    let saved_permissions =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM browser_permissions")
            .fetch_one(db)
            .await?;
    Ok(BrowserPrivacySummary {
        bookmarks,
        history_entries,
        saved_permissions,
    })
}

fn canonical_http_url(raw: &str) -> Result<String, BrowserError> {
    let parsed = tauri::Url::parse(raw.trim())
        .map_err(|error| BrowserError::InvalidUrl(format!("invalid browser URL: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(BrowserError::InvalidUrl(
            "only absolute HTTP(S) URLs can be stored".into(),
        ));
    }
    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::init_test_db;

    #[tokio::test]
    async fn bookmarks_and_history_are_isolated_metadata() {
        let db = init_test_db().await.pool;
        let bookmark = add_bookmark(&db, "https://example.com/mod", Some("Example"), None)
            .await
            .unwrap();
        record_history(&db, "https://example.com/mod", Some("Example"), None)
            .await
            .unwrap();
        record_history(
            &db,
            "https://example.com/mod",
            Some("Example updated"),
            None,
        )
        .await
        .unwrap();

        let bookmarks = list_bookmarks(&db).await.unwrap();
        assert!(bookmarks
            .iter()
            .any(|candidate| candidate.id == bookmark.id));
        let history = list_history(&db, 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].visit_count, 2);
        assert_eq!(history[0].title, "Example updated");

        clear_history(&db).await.unwrap();
        assert!(list_history(&db, 10).await.unwrap().is_empty());
        assert!(list_bookmarks(&db)
            .await
            .unwrap()
            .iter()
            .any(|candidate| candidate.id == bookmark.id));
    }

    #[tokio::test]
    async fn default_gamebanana_bookmark_is_editable_and_deletable() {
        let db = init_test_db().await.pool;
        let default_bookmark = list_bookmarks(&db)
            .await
            .unwrap()
            .into_iter()
            .find(|bookmark| bookmark.url == "https://gamebanana.com/")
            .expect("default GameBanana bookmark");

        let updated = update_bookmark(
            &db,
            &default_bookmark.id,
            "https://gamebanana.com/mods",
            "GameBanana Mods",
        )
        .await
        .unwrap();
        assert_eq!(updated.id, default_bookmark.id);
        assert_eq!(updated.url, "https://gamebanana.com/mods");
        assert_eq!(updated.title, "GameBanana Mods");

        delete_bookmark(&db, &updated.id).await.unwrap();
        assert!(list_bookmarks(&db)
            .await
            .unwrap()
            .iter()
            .all(|bookmark| bookmark.id != updated.id));
    }

    #[tokio::test]
    async fn bookmark_edits_keep_the_existing_bookmark_identity() {
        let db = init_test_db().await.pool;
        let bookmark = add_bookmark(&db, "https://example.com/mod", Some("Example"), None)
            .await
            .unwrap();

        let updated = update_bookmark(
            &db,
            &bookmark.id,
            "https://example.com/renamed-mod",
            "Renamed example",
        )
        .await
        .unwrap();

        assert_eq!(updated.id, bookmark.id);
        assert_eq!(updated.url, "https://example.com/renamed-mod");
        assert_eq!(updated.title, "Renamed example");

        let duplicate = add_bookmark(&db, "https://example.com/other", Some("Other"), None)
            .await
            .unwrap();
        assert!(matches!(
            update_bookmark(&db, &updated.id, &duplicate.url, "Duplicate").await,
            Err(BrowserError::InvalidSetting(_))
        ));
    }

    #[tokio::test]
    async fn session_tabs_are_bounded_and_require_http_urls() {
        let db = init_test_db().await.pool;
        let tabs = (0..13)
            .map(|position| BrowserSessionTab {
                position,
                url: format!("https://example.com/{position}"),
                title: String::new(),
                active: position == 0,
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            save_session_tabs(&db, "game-1", &tabs).await,
            Err(BrowserError::InvalidSetting(_))
        ));

        assert!(matches!(
            add_bookmark(&db, "file:///not-allowed", None, None).await,
            Err(BrowserError::InvalidUrl(_))
        ));
    }
}
