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

#[cfg(test)]
mod tests {
    use super::{get_status, set_pin, verify_pin};
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
}
