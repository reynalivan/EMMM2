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

/// Outcome of a PIN attempt.
///
/// `NoPinConfigured` is reported rather than folded into "accepted": whether a
/// missing PIN means *allow* (an unlock prompt has nothing to unlock) or *deny*
/// (a privilege gate has nothing to prove against) is the gate's decision, not
/// the verifier's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinVerdict {
    NoPinConfigured,
    Accepted,
    Rejected { attempts_remaining: i32 },
    LockedOut { seconds_remaining: i32 },
}

/// Verify a PIN attempt, recording failed attempts and enforcing lockout.
///
/// The verdict carries the attempt/lockout counters, so callers do not need to
/// re-read the status to find out why an attempt failed.
pub async fn verify_pin(pool: &SqlitePool, pin: &str) -> Result<PinVerdict, PinError> {
    let config = pin_repo::get(pool).await?;

    if !config.has_pin() {
        pin_repo::reset_failed_attempts(pool).await?;
        return Ok(PinVerdict::NoPinConfigured);
    }

    let lockout_seconds = config.lockout_seconds_remaining();
    if lockout_seconds > 0 {
        return Ok(PinVerdict::LockedOut {
            seconds_remaining: lockout_seconds,
        });
    }

    if config.lockout_until.is_some() {
        pin_repo::reset_failed_attempts(pool).await?;
    }

    if verify_hash(pin, config.pin_hash.as_deref().unwrap_or_default()) {
        pin_repo::reset_failed_attempts(pool).await?;
        return Ok(PinVerdict::Accepted);
    }

    let failed_attempts = config.failed_attempts.saturating_add(1);
    if failed_attempts >= i32::from(MAX_PIN_ATTEMPTS) {
        pin_repo::set_lockout_seconds(pool, PIN_LOCKOUT_SECONDS).await?;
        return Ok(PinVerdict::LockedOut {
            seconds_remaining: PIN_LOCKOUT_SECONDS,
        });
    }

    pin_repo::set_failed_attempts(pool, failed_attempts).await?;
    Ok(PinVerdict::Rejected {
        attempts_remaining: i32::from(MAX_PIN_ATTEMPTS).saturating_sub(failed_attempts),
    })
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
#[path = "tests/pin_service_tests.rs"]
mod tests;
