//! Homepage preference and URL scheme rules.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::modules::browser::adapters::sqlite::browser;
use crate::shared::errors::BrowserError;

const DEFAULT_RETENTION_DAYS: i64 = 30;
const LEGACY_DATABASE_DEFAULT_RETENTION_DAYS: i64 = 3;
const MIN_RETENTION_DAYS: i64 = 1;
const MAX_RETENTION_DAYS: i64 = 365;
const RETENTION_MIGRATION_MARKER: &str = "retention_days_migration_v1";

/// Fetch the configured homepage URL for one game.
/// Falls back to `https://www.google.com` if not set.
pub async fn get_homepage(db: &SqlitePool, game_id: &str) -> String {
    sqlx::query_scalar("SELECT homepage_url FROM browser_game_settings WHERE game_id = ?")
        .bind(game_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "https://www.google.com".to_string())
}

/// Save a new homepage URL for one game.
pub async fn set_homepage(db: &SqlitePool, game_id: &str, url: &str) -> Result<(), BrowserError> {
    validate_http_url(url)?;
    sqlx::query(
        "INSERT INTO browser_game_settings (game_id, homepage_url, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT(game_id) DO UPDATE SET homepage_url = excluded.homepage_url, updated_at = excluded.updated_at",
    )
    .bind(game_id)
    .bind(url)
    .bind(Utc::now().timestamp())
    .execute(db)
    .await?;
    Ok(())
}

/// Read the configured retention period, falling back if a legacy value is invalid.
pub async fn get_retention_days(db: &SqlitePool) -> Result<i64, BrowserError> {
    let configured = browser::get_retention_days(db).await?;
    Ok(configured
        .filter(|days| is_valid_retention_days(*days))
        .unwrap_or(DEFAULT_RETENTION_DAYS))
}

/// Preserve a valid retention value from the legacy browser store during the
/// one-time move away from the legacy database default. The marker prevents a
/// stale local value from overwriting a preference saved through the new UI.
pub async fn get_or_migrate_retention_days(
    db: &SqlitePool,
    legacy_retention_days: Option<i64>,
) -> Result<i64, BrowserError> {
    if browser::get_setting(db, RETENTION_MIGRATION_MARKER)
        .await?
        .is_some()
    {
        return get_retention_days(db).await;
    }

    let configured = browser::get_retention_days(db).await?;
    let retention_days = match configured {
        Some(days)
            if is_valid_retention_days(days) && days != LEGACY_DATABASE_DEFAULT_RETENTION_DAYS =>
        {
            days
        }
        Some(_) | None => legacy_retention_days
            .filter(|days| is_valid_retention_days(*days))
            .unwrap_or(DEFAULT_RETENTION_DAYS),
    };

    set_retention_days(db, retention_days).await?;
    browser::set_setting(db, RETENTION_MIGRATION_MARKER, "complete").await?;
    Ok(retention_days)
}

/// Store a retention period that is safe for the cleanup job to consume.
pub async fn set_retention_days(db: &SqlitePool, days: i64) -> Result<(), BrowserError> {
    if !is_valid_retention_days(days) {
        return Err(BrowserError::InvalidSetting(format!(
            "retention days must be between {MIN_RETENTION_DAYS} and {MAX_RETENTION_DAYS}"
        )));
    }

    browser::set_setting(db, "retention_days", &days.to_string()).await?;
    Ok(())
}

fn is_valid_retention_days(days: i64) -> bool {
    (MIN_RETENTION_DAYS..=MAX_RETENTION_DAYS).contains(&days)
}

/// Validate that URL is http or https only.
pub fn validate_http_url(url: &str) -> Result<(), BrowserError> {
    let lower = url.trim().to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(());
    }
    Err(BrowserError::InvalidUrl(format!(
        "only http:// and https:// are allowed, got '{url}'"
    )))
}

/// Auto-prepend https:// if user typed a URL without a scheme.
pub fn normalize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::init_test_db;

    #[test]
    fn validate_http_url_accepts_only_http_schemes_case_insensitively() {
        assert!(validate_http_url("http://example.com").is_ok());
        assert!(validate_http_url("  HTTPS://Example.com  ").is_ok());

        for bad in [
            "ftp://example.com",
            "file:///C:/secrets.txt",
            "javascript:alert(1)",
            "example.com",
            "",
        ] {
            assert!(
                matches!(
                    validate_http_url(bad).unwrap_err(),
                    BrowserError::InvalidUrl(_)
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn normalize_url_prepends_https_only_when_a_scheme_is_missing() {
        assert_eq!(normalize_url("  example.com "), "https://example.com");
        assert_eq!(normalize_url("http://a.test"), "http://a.test");
        assert_eq!(normalize_url(" https://a.test "), "https://a.test");
        // Only lowercase http(s) counts as "already schemed" here.
        assert_eq!(normalize_url("HTTP://a.test"), "https://HTTP://a.test");
    }

    #[tokio::test]
    async fn get_homepage_falls_back_to_google_when_unset() {
        let db = init_test_db().await.pool;
        assert_eq!(get_homepage(&db, "game-1").await, "https://www.google.com");
    }

    #[tokio::test]
    async fn set_homepage_upserts_the_stored_value() {
        let db = init_test_db().await.pool;

        set_homepage(&db, "game-1", "https://gamebanana.com")
            .await
            .unwrap();
        assert_eq!(get_homepage(&db, "game-1").await, "https://gamebanana.com");

        set_homepage(&db, "game-2", "http://localhost:1420")
            .await
            .unwrap();
        assert_eq!(get_homepage(&db, "game-1").await, "https://gamebanana.com");
        assert_eq!(get_homepage(&db, "game-2").await, "http://localhost:1420");
    }

    #[tokio::test]
    async fn set_homepage_rejects_a_bad_scheme_without_writing() {
        let db = init_test_db().await.pool;
        set_homepage(&db, "game-1", "https://ok.test")
            .await
            .unwrap();

        let err = set_homepage(&db, "game-1", "javascript:alert(1)")
            .await
            .unwrap_err();

        assert!(matches!(err, BrowserError::InvalidUrl(_)));
        assert_eq!(get_homepage(&db, "game-1").await, "https://ok.test");
    }

    #[tokio::test]
    async fn retention_days_round_trip_through_the_browser_database() {
        let db = init_test_db().await.pool;

        set_retention_days(&db, 14).await.unwrap();
        assert_eq!(get_retention_days(&db).await.unwrap(), 14);

        assert!(matches!(
            set_retention_days(&db, 0).await.unwrap_err(),
            BrowserError::InvalidSetting(_)
        ));
        assert_eq!(get_retention_days(&db).await.unwrap(), 14);
    }

    #[tokio::test]
    async fn migration_preserves_legacy_retention_and_keeps_existing_database_setting() {
        let db = init_test_db().await.pool;

        assert_eq!(
            get_or_migrate_retention_days(&db, Some(365)).await.unwrap(),
            365
        );
        assert_eq!(get_retention_days(&db).await.unwrap(), 365);

        set_retention_days(&db, 14).await.unwrap();
        assert_eq!(
            get_or_migrate_retention_days(&db, Some(1)).await.unwrap(),
            14
        );
    }
}
