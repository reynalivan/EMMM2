use crate::common::sync::lock;
use crate::domain::errors::AppError;

use super::pin_guard::{validate_pin_format, PinVerifyStatus};
use super::ConfigService;
use crate::services::pin_service::PinVerdict;

/// Mirrors `pin_service::MAX_PIN_ATTEMPTS` for the "nothing to refuse" answer.
const MAX_PIN_ATTEMPTS: u8 = 5;

impl ConfigService {
    pub fn verify_pin_status(&self, pin: &str) -> PinVerifyStatus {
        match self.pin_verdict(pin) {
            // An unlock prompt with no PIN set has nothing to refuse.
            Some(PinVerdict::NoPinConfigured) | Some(PinVerdict::Accepted) => PinVerifyStatus {
                valid: true,
                attempts_remaining: MAX_PIN_ATTEMPTS,
                locked_seconds_remaining: 0,
            },
            Some(PinVerdict::Rejected { attempts_remaining }) => PinVerifyStatus {
                valid: false,
                attempts_remaining: attempts_remaining.max(0) as u8,
                locked_seconds_remaining: 0,
            },
            Some(PinVerdict::LockedOut { seconds_remaining }) => PinVerifyStatus {
                valid: false,
                attempts_remaining: 0,
                locked_seconds_remaining: seconds_remaining.max(0) as u64,
            },
            None => PinVerifyStatus {
                valid: false,
                attempts_remaining: 0,
                locked_seconds_remaining: 0,
            },
        }
    }

    /// Whether `pin` proves the user may see beyond the Safe Mode corridor.
    ///
    /// Deliberately stricter than [`Self::verify_pin_status`]: a configured PIN
    /// must be presented and matched. With no PIN set there is nothing to
    /// prove, so elevation is refused rather than granted.
    pub fn pin_grants_elevation(&self, pin: &str) -> bool {
        matches!(self.pin_verdict(pin), Some(PinVerdict::Accepted))
    }

    /// `None` when the verification itself failed (e.g. the DB is unavailable),
    /// which every caller must treat as "not proven".
    fn pin_verdict(&self, pin: &str) -> Option<PinVerdict> {
        let pool = self.pool.clone();
        Self::run_async(async move { crate::services::pin_service::verify_pin(&pool, pin).await })
            .map_err(|error| log::warn!("PIN verification unavailable: {error}"))
            .ok()
    }

    pub fn set_pin(&self, pin: &str) -> Result<(), AppError> {
        validate_pin_format(pin)?;

        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_string();

        let mut settings = lock(&self.settings).clone();
        settings.safe_mode.pin_hash = Some(password_hash);

        self.save_settings(settings)?;
        let pool = self.pool.clone();
        Self::run_async(async {
            crate::services::pin_service::set_pin(&pool, pin, None)
                .await
                .map_err(AppError::from)
        })?;

        Ok(())
    }

    /// Validates the recovery code. If valid, clears the PIN and recovery code,
    /// allowing the user to set a new PIN without knowing the old one.
    /// Returns `true` if reset succeeded, `Err` on internal errors.
    pub fn reset_pin_with_recovery_code(&self, code: &str) -> Result<bool, AppError> {
        use sha2::{Digest, Sha256};

        let stored_hash = {
            let settings = lock(&self.settings);
            settings.safe_mode.recovery_code_hash.clone()
        };

        let Some(stored_hash) = stored_hash else {
            // No recovery code configured for this installation
            return Ok(false);
        };

        // Normalise input (uppercase, trim whitespace)
        let code_normalised = code.trim().to_uppercase();

        let mut hasher = Sha256::new();
        hasher.update(code_normalised.as_bytes());
        let input_hash = format!("{:x}", hasher.finalize());

        if input_hash != stored_hash {
            return Ok(false);
        }

        // Valid — clear PIN and recovery code
        let mut settings = lock(&self.settings).clone();
        settings.safe_mode.pin_hash = None;
        settings.safe_mode.recovery_code_hash = None;
        settings.safe_mode.failed_attempts = None;
        settings.safe_mode.lockout_until_ts = None;
        self.save_settings(settings)?;
        let pool = self.pool.clone();
        Self::run_async(async move {
            crate::repo::pin_repo::clear_pin(&pool)
                .await
                .map_err(AppError::from)
        })?;

        Ok(true)
    }
}
