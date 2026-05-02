use sqlx::SqlitePool;

use crate::domain::errors::PinError;
use crate::domain::pin::PinStatus;
use crate::repo::pin_repo;

const MAX_PIN_ATTEMPTS: u8 = 5;
const PIN_LOCKOUT_SECONDS: i32 = 60;

/// Get the PIN status (safe for frontend — no hashes).
pub async fn get_status(pool: &SqlitePool) -> Result<PinStatus, PinError> {
    let config = pin_repo::get(pool).await?;
    let lockout_seconds_remaining = config.lockout_seconds_remaining();
    let is_locked = lockout_seconds_remaining > 0;
    let attempts_remaining = if is_locked {
        0
    } else {
        i32::from(MAX_PIN_ATTEMPTS).saturating_sub(config.failed_attempts)
    };

    Ok(PinStatus {
        has_pin: config.has_pin(),
        is_locked,
        attempts_remaining,
        lockout_seconds_remaining,
    })
}

/// Set a new PIN. Hashes the PIN with Argon2 before storing.
pub async fn set_pin(
    pool: &SqlitePool,
    pin: &str,
    recovery_code: Option<&str>,
) -> Result<(), PinError> {
    let pin_hash = hash_pin(pin);
    let recovery_hash = recovery_code.map(hash_pin);

    pin_repo::set_pin(pool, &pin_hash, recovery_hash.as_deref()).await?;
    pin_repo::reset_failed_attempts(pool).await?;
    Ok(())
}

/// Verify a PIN attempt. Returns Ok(true) if correct.
/// Records failed attempts and enforces lockout.
pub async fn verify_pin(pool: &SqlitePool, pin: &str) -> Result<bool, PinError> {
    let config = pin_repo::get(pool).await?;

    // No PIN set — verification always passes
    if !config.has_pin() {
        pin_repo::reset_failed_attempts(pool).await?;
        return Ok(true);
    }

    if config.is_locked() {
        return Ok(false);
    }

    if config.lockout_until.is_some() {
        pin_repo::reset_failed_attempts(pool).await?;
    }

    if verify_hash(pin, config.pin_hash.as_deref().unwrap_or_default()) {
        pin_repo::reset_failed_attempts(pool).await?;
        return Ok(true);
    }

    let failed_attempts = config.failed_attempts.saturating_add(1);
    if failed_attempts >= i32::from(MAX_PIN_ATTEMPTS) {
        pin_repo::set_lockout_seconds(pool, PIN_LOCKOUT_SECONDS).await?;
        return Ok(false);
    }

    pin_repo::set_failed_attempts(pool, failed_attempts).await?;
    Ok(false)
}

/// Verify a recovery code. Same logic as PIN verification.
pub async fn verify_recovery(pool: &SqlitePool, recovery_code: &str) -> Result<bool, PinError> {
    let config = pin_repo::get(pool).await?;

    let recovery_hash = match config.recovery_hash {
        Some(ref h) => h,
        None => return Ok(false), // No recovery code set
    };

    let is_valid = verify_hash(recovery_code, recovery_hash);

    if is_valid {
        // Recovery clears the PIN entirely
        pin_repo::clear_pin(pool).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Clear the PIN (remove protection).
pub async fn clear_pin(pool: &SqlitePool) -> Result<(), PinError> {
    pin_repo::clear_pin(pool).await?;
    Ok(())
}

/// Check if a PIN is set.
pub async fn has_pin(pool: &SqlitePool) -> Result<bool, PinError> {
    let config = pin_repo::get(pool).await?;
    Ok(config.has_pin())
}

/// Check if the UI should be locked on boot.
/// Returns true if Safe Mode is DISABLED and a PIN is set.
pub async fn check_boot_security(pool: &SqlitePool, is_safe_mode: bool) -> Result<bool, PinError> {
    let config = pin_repo::get(pool).await?;
    // Requirement: IF Unsafe AND PIN is set -> LOCK
    Ok(!is_safe_mode && config.has_pin())
}

/// Verify a PIN or recovery code against an Argon2 hash.
fn verify_hash(secret: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed = match PasswordHash::new(hash) {
        Ok(value) => value,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::{check_boot_security, get_status, set_pin, verify_pin};
    use crate::repo::pin_repo;
    use crate::test_utils::init_test_db;

    #[tokio::test]
    async fn verify_pin_persists_sixty_second_lockout_in_db() {
        let ctx = init_test_db().await;

        set_pin(&ctx.pool, "123456", None).await.expect("set pin");

        for _ in 0..5 {
            let valid = verify_pin(&ctx.pool, "000000").await.expect("verify pin");
            assert!(!valid);
        }

        let status = get_status(&ctx.pool).await.expect("get status");
        assert!(status.is_locked);
        assert!(status.lockout_seconds_remaining > 0);
        assert!(status.lockout_seconds_remaining <= 60);

        let db_status = pin_repo::get(&ctx.pool).await.expect("pin config");
        assert_eq!(db_status.failed_attempts, 0);
        assert!(
            db_status.lockout_until.is_some(),
            "lockout must survive service restart through pin_config"
        );
    }

    #[tokio::test]
    async fn check_boot_security_locks_only_for_unsafe_with_pin() {
        let ctx = init_test_db().await;

        set_pin(&ctx.pool, "123456", None).await.expect("set pin");

        let safe_locked = check_boot_security(&ctx.pool, true)
            .await
            .expect("safe boot status");
        let unsafe_locked = check_boot_security(&ctx.pool, false)
            .await
            .expect("unsafe boot status");

        assert!(!safe_locked);
        assert!(unsafe_locked);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Hash a PIN using Argon2id.
fn hash_pin(pin: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(pin.as_bytes(), &salt)
        .expect("Argon2 hashing should not fail")
        .to_string()
}
