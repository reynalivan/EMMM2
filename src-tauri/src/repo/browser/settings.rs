//! `browser_settings` key/value persistence.

use sqlx::SqlitePool;

/// Read a raw `browser_settings` value. `None` when the key is absent.
pub async fn get_setting(db: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!("SELECT value FROM browser_settings WHERE key = ?", key)
        .fetch_optional(db)
        .await
}

/// Upsert a `browser_settings` value.
pub async fn set_setting(db: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO browser_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        key,
        value
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Read `retention_days` with SQL `CAST` semantics (preserved from the original query).
pub async fn get_retention_days(db: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT CAST(value AS INTEGER) FROM browser_settings WHERE key = 'retention_days'"
    )
    .fetch_optional(db)
    .await
}
