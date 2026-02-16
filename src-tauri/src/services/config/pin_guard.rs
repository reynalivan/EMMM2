use serde::Serialize;
use std::time::{Duration, SystemTime};

const MAX_PIN_ATTEMPTS: u8 = 5;
const PIN_LOCKOUT_SECONDS: u64 = 60;

#[derive(Debug, Clone, Serialize)]
pub struct PinVerifyStatus {
    pub valid: bool,
    pub attempts_remaining: u8,
    pub locked_seconds_remaining: u64,
}

#[derive(Debug, Default)]
pub struct PinGuardState {
    failed_attempts: u8,
    locked_until: Option<SystemTime>,
}

impl PinGuardState {
    pub fn reset(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = None;
    }

    pub fn verify(&mut self, pin: &str, pin_hash: Option<&str>) -> PinVerifyStatus {
        let Some(hash) = pin_hash else {
            return PinVerifyStatus {
                valid: true,
                attempts_remaining: MAX_PIN_ATTEMPTS,
                locked_seconds_remaining: 0,
            };
        };

        if let Some(until) = self.locked_until {
            let now = SystemTime::now();
            if now < until {
                let remaining = until
                    .duration_since(now)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs()
                    .max(1);
                return PinVerifyStatus {
                    valid: false,
                    attempts_remaining: 0,
                    locked_seconds_remaining: remaining,
                };
            }
            self.reset();
        }

        if verify_hash(hash, pin) {
            self.reset();
            return PinVerifyStatus {
                valid: true,
                attempts_remaining: MAX_PIN_ATTEMPTS,
                locked_seconds_remaining: 0,
            };
        }

        self.failed_attempts = self.failed_attempts.saturating_add(1);
        if self.failed_attempts >= MAX_PIN_ATTEMPTS {
            self.failed_attempts = 0;
            self.locked_until = Some(SystemTime::now() + Duration::from_secs(PIN_LOCKOUT_SECONDS));
            return PinVerifyStatus {
                valid: false,
                attempts_remaining: 0,
                locked_seconds_remaining: PIN_LOCKOUT_SECONDS,
            };
        }

        PinVerifyStatus {
            valid: false,
            attempts_remaining: MAX_PIN_ATTEMPTS - self.failed_attempts,
            locked_seconds_remaining: 0,
        }
    }
}

pub fn validate_pin_format(pin: &str) -> Result<(), String> {
    if pin.len() != 6 || !pin.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("PIN must be exactly 6 digits".to_string());
    }

    Ok(())
}

fn verify_hash(hash: &str, pin: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(pin.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pin_format() {
        assert!(validate_pin_format("123456").is_ok());
        assert!(validate_pin_format("12345").is_err());
        assert!(validate_pin_format("1234ab").is_err());
    }
}
