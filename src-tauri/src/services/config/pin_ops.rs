use crate::domain::errors::AppError;

use super::pin_guard::{validate_pin_format, PinVerifyStatus};
use super::ConfigService;

impl ConfigService {
    pub fn verify_pin_status(&self, pin: &str) -> PinVerifyStatus {
        let pool = self.pool.clone();
        Self::run_async(async move {
            let before = crate::services::pin_service::get_status(&pool)
                .await
                .map_err(|error| error.to_string())?;
            if before.is_locked {
                return Ok(PinVerifyStatus {
                    valid: false,
                    attempts_remaining: 0,
                    locked_seconds_remaining: before.lockout_seconds_remaining.max(0) as u64,
                });
            }

            let valid = crate::services::pin_service::verify_pin(&pool, pin)
                .await
                .map_err(|error| error.to_string())?;
            let after = crate::services::pin_service::get_status(&pool)
                .await
                .map_err(|error| error.to_string())?;

            Ok::<PinVerifyStatus, String>(PinVerifyStatus {
                valid,
                attempts_remaining: after.attempts_remaining.max(0) as u8,
                locked_seconds_remaining: after.lockout_seconds_remaining.max(0) as u64,
            })
        })
        .unwrap_or(PinVerifyStatus {
            valid: false,
            attempts_remaining: 0,
            locked_seconds_remaining: 0,
        })
    }

    pub fn verify_pin(&self, pin: &str) -> bool {
        self.verify_pin_status(pin).valid
    }

    pub fn set_pin(&self, pin: &str) -> Result<(), AppError> {
        validate_pin_format(pin).map_err(AppError::Validation)?;

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

        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
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
            let settings = self
                .settings
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
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
