//! Homepage preference and URL scheme rules.

use sqlx::SqlitePool;

use crate::domain::errors::BrowserError;
use crate::repo::browser_repo;

/// Fetch the configured homepage URL from `browser_settings` table.
/// Falls back to `https://www.google.com` if not set.
pub async fn get_homepage(db: &SqlitePool) -> String {
    browser_repo::get_setting(db, "homepage_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "https://www.google.com".to_string())
}

/// Save a new homepage URL to `browser_settings`.
pub async fn set_homepage(db: &SqlitePool, url: &str) -> Result<(), BrowserError> {
    validate_http_url(url)?;
    browser_repo::set_setting(db, "homepage_url", url).await?;
    Ok(())
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
        assert_eq!(get_homepage(&db).await, "https://www.google.com");
    }

    #[tokio::test]
    async fn set_homepage_upserts_the_stored_value() {
        let db = init_test_db().await.pool;

        set_homepage(&db, "https://gamebanana.com").await.unwrap();
        assert_eq!(get_homepage(&db).await, "https://gamebanana.com");

        set_homepage(&db, "http://localhost:1420").await.unwrap();
        assert_eq!(get_homepage(&db).await, "http://localhost:1420");
    }

    #[tokio::test]
    async fn set_homepage_rejects_a_bad_scheme_without_writing() {
        let db = init_test_db().await.pool;
        set_homepage(&db, "https://ok.test").await.unwrap();

        let err = set_homepage(&db, "javascript:alert(1)").await.unwrap_err();

        assert!(matches!(err, BrowserError::InvalidUrl(_)));
        assert_eq!(get_homepage(&db).await, "https://ok.test");
    }
}
