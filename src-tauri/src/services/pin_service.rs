use sqlx::SqlitePool;

use crate::domain::errors::PinError;
use crate::domain::pin::PinStatus;
use crate::repo::pin_repo;

// ---------------------------------------------------------------------------
// pin_service — Business logic for PIN security
// ---------------------------------------------------------------------------

/// Maximum failed PIN attempts before lockout.
pub const MAX_FAILED_ATTEMPTS: i32 = 5;
/// Lockout duration in minutes after max failed attempts.
const LOCKOUT_MINUTES: i32 = 15;

/// Get the PIN status (safe for frontend — no hashes).
pub async fn get_status(pool: &SqlitePool) -> Result<PinStatus, PinError> {
    let config = pin_repo::get(pool).await?;

    Ok(PinStatus {
        has_pin: config.has_pin(),
        is_locked: config.is_locked(),
        attempts_remaining: (MAX_FAILED_ATTEMPTS - config.failed_attempts).max(0),
        lockout_seconds_remaining: config.lockout_seconds_remaining(),
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

    pin_repo::set_pin(pool, &pin_hash, recovery_hash.as_deref()).await
}

/// Verify a PIN attempt. Returns Ok(true) if correct.
/// Records failed attempts and enforces lockout.
pub async fn verify_pin(pool: &SqlitePool, pin: &str) -> Result<bool, PinError> {
    let config = pin_repo::get(pool).await?;

    // Check lockout
    if config.is_locked() {
        return Err(PinError::Locked(config.lockout_until.unwrap_or_default()));
    }

    // No PIN set — verification always passes
    if !config.has_pin() {
        return Ok(true);
    }

    // Verify against stored hash
    let is_valid = verify_hash(pin, config.pin_hash.as_deref().unwrap_or_default());

    if is_valid {
        pin_repo::reset_failed_attempts(pool).await?;
        Ok(true)
    } else {
        pin_repo::record_failed_attempt(pool, MAX_FAILED_ATTEMPTS, LOCKOUT_MINUTES).await?;
        Ok(false)
    }
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
    pin_repo::clear_pin(pool).await
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

/// Verify a PIN against an Argon2 hash.
fn verify_hash(pin: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(pin.as_bytes(), &parsed)
        .is_ok()
}
