use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

/// DTO for a download session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadSessionDto {
    pub id: String,
    pub source: String,
    pub submission_id: Option<String>,
    pub mod_title: Option<String>,
    pub profile_url: Option<String>,
    pub game_id: Option<String>,
    pub expected_keywords: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// Create a new download session for a Discover Hub "Download" click.
pub async fn create_session(
    db: &SqlitePool,
    source: &str, // "gamebanana" | "adhoc"
    submission_id: Option<&str>,
    mod_title: Option<&str>,
    profile_url: Option<&str>,
    game_id: Option<&str>,
    expected_keywords: Option<&[&str]>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let keywords_json = expected_keywords
        .map(|kws| serde_json::to_string(&kws).unwrap_or_else(|_| "[]".to_string()));

    sqlx::query!(
        r#"INSERT INTO download_sessions
           (id, source, submission_id, mod_title, profile_url, game_id, expected_keywords, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        id, source, submission_id, mod_title, profile_url, game_id, keywords_json, now
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB insert session failed: {e}"))?;

    Ok(id)
}

/// Mark a session as `downloading` when the first file is intercepted.
pub async fn mark_session_downloading(db: &SqlitePool, session_id: &str) -> Result<(), String> {
    sqlx::query!(
        "UPDATE download_sessions SET status = 'downloading' WHERE id = ?",
        session_id
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB update session failed: {e}"))?;
    Ok(())
}

/// Mark a session as `complete` when all expected downloads are done.
pub async fn mark_session_complete(db: &SqlitePool, session_id: &str) -> Result<(), String> {
    sqlx::query!(
        "UPDATE download_sessions SET status = 'complete' WHERE id = ?",
        session_id
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB update session complete failed: {e}"))?;
    Ok(())
}

/// Return the most recent `awaiting_download` session within the last 30 minutes.
/// Used to auto-correlate adhoc browser downloads with an active session.
pub async fn find_recent_active_session(db: &SqlitePool) -> Option<String> {
    sqlx::query_scalar!(
        r#"SELECT id FROM download_sessions
           WHERE status = 'awaiting_download'
             AND created_at >= datetime('now', '-30 minutes')
           ORDER BY created_at DESC
           LIMIT 1"#
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten()
}
