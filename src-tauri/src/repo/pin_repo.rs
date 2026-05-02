use sqlx::{Row, SqlitePool};

use crate::domain::errors::PinError;
use crate::domain::pin::PinConfig;

// ---------------------------------------------------------------------------
// pin_repo — CRUD for the `pin_config` singleton table
// Uses runtime queries (not sqlx macros) because the new table doesn't exist
// in the compile-time DB during the migration coexistence period.
// ---------------------------------------------------------------------------

/// Get the current PIN configuration.
pub async fn get(pool: &SqlitePool) -> Result<PinConfig, PinError> {
    let row = sqlx::query(
        r#"SELECT pin_hash, recovery_hash, failed_attempts, lockout_until, updated_at
        FROM pin_config WHERE id = 1"#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|r| PinConfig {
            pin_hash: r.get("pin_hash"),
            recovery_hash: r.get("recovery_hash"),
            failed_attempts: r.get("failed_attempts"),
            lockout_until: r.get("lockout_until"),
            updated_at: r.get("updated_at"),
        })
        .unwrap_or_else(|| PinConfig {
            pin_hash: None,
            recovery_hash: None,
            failed_attempts: 0,
            lockout_until: None,
            updated_at: String::new(),
        }))
}

/// Set the PIN hash (and optional recovery hash).
pub async fn set_pin(
    pool: &SqlitePool,
    pin_hash: &str,
    recovery_hash: Option<&str>,
) -> Result<(), PinError> {
    sqlx::query(
        r#"UPDATE pin_config SET
            pin_hash = ?,
            recovery_hash = ?,
            failed_attempts = 0,
            lockout_until = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = 1"#,
    )
    .bind(pin_hash)
    .bind(recovery_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear the PIN (remove protection).
pub async fn clear_pin(pool: &SqlitePool) -> Result<(), PinError> {
    sqlx::query(
        r#"UPDATE pin_config SET
            pin_hash = NULL,
            recovery_hash = NULL,
            failed_attempts = 0,
            lockout_until = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = 1"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Increment failed attempts and set lockout if threshold reached.
pub async fn record_failed_attempt(
    pool: &SqlitePool,
    max_attempts: i32,
    lockout_minutes: i32,
) -> Result<i32, PinError> {
    sqlx::query(
        r#"UPDATE pin_config SET
            failed_attempts = failed_attempts + 1,
            lockout_until = CASE
                WHEN failed_attempts + 1 >= ?
                THEN datetime('now', '+' || ? || ' minutes')
                ELSE lockout_until
            END,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = 1"#,
    )
    .bind(max_attempts)
    .bind(lockout_minutes)
    .execute(pool)
    .await?;

    let count: i32 = sqlx::query_scalar("SELECT failed_attempts FROM pin_config WHERE id = 1")
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn set_failed_attempts(pool: &SqlitePool, failed_attempts: i32) -> Result<(), PinError> {
    sqlx::query(
        r#"UPDATE pin_config SET
            failed_attempts = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = 1"#,
    )
    .bind(failed_attempts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_lockout_seconds(pool: &SqlitePool, seconds: i32) -> Result<(), PinError> {
    sqlx::query(
        r#"UPDATE pin_config SET
            failed_attempts = 0,
            lockout_until = datetime('now', '+' || ? || ' seconds'),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = 1"#,
    )
    .bind(seconds)
    .execute(pool)
    .await?;
    Ok(())
}

/// Reset failed attempts (after successful verification).
pub async fn reset_failed_attempts(pool: &SqlitePool) -> Result<(), PinError> {
    sqlx::query(
        r#"UPDATE pin_config SET
            failed_attempts = 0,
            lockout_until = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = 1"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
