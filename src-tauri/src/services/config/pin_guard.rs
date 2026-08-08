//! Safe Mode PIN input validation.
//!
//! Lockout state and hash verification are NOT here — they live in
//! `services::pin_service` over the `pin_config` table, which is the
//! authoritative store. This module owns format validation only.

use crate::domain::errors::AppError;
use serde::Serialize;

/// Safe Mode PINs are a fixed-length numeric code.
const PIN_LENGTH: usize = 6;

#[derive(Debug, Clone, Serialize)]
pub struct PinVerifyStatus {
    pub valid: bool,
    pub attempts_remaining: u8,
    pub locked_seconds_remaining: u64,
}

pub fn validate_pin_format(pin: &str) -> Result<(), AppError> {
    if pin.len() != PIN_LENGTH || !pin.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(AppError::Internal(format!(
            "PIN must be exactly {PIN_LENGTH} digits"
        )));
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/pin_guard_tests.rs"]
mod tests;
