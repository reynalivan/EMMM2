use super::*;

fn hash_pin_for_test(pin: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

// TC-30-011: Validating the PIN format
#[test]
fn test_validate_pin_format() {
    assert!(validate_pin_format("123456").is_ok());
    assert!(validate_pin_format("12345").is_err()); // Too short
    assert!(validate_pin_format("1234567").is_err()); // Too long
    assert!(validate_pin_format("1234ab").is_err()); // Non-numeric
}

// TC-30-015: Successful verify resets lockout counter
#[test]
fn test_verify_success_resets_attempts() {
    let mut guard = PinGuardState::default();
    let hash = hash_pin_for_test("123456");

    // Fail 3 times
    for _ in 0..3 {
        guard.verify("000000", Some(&hash));
    }

    // 4th attempt succeeds
    let status = guard.verify("123456", Some(&hash));

    assert!(status.valid);
    assert_eq!(status.attempts_remaining, MAX_PIN_ATTEMPTS);

    // Internal state should be reset
    let (failed, _locked_until) = guard.snapshot();
    assert_eq!(failed, 0);
}

// TC-30-016: Failing repeatedly causes lockout
#[test]
fn test_lockout_on_max_attempts() {
    let mut guard = PinGuardState::default();
    let hash = hash_pin_for_test("123456");

    // Fail 4 times (max is 5)
    for i in 1..=4 {
        let status = guard.verify("000000", Some(&hash));
        assert!(!status.valid);
        assert_eq!(status.attempts_remaining, MAX_PIN_ATTEMPTS - i);
        assert_eq!(status.locked_seconds_remaining, 0);
    }

    // 5th fail -> Lockout
    let status = guard.verify("000000", Some(&hash));
    assert!(!status.valid);
    assert_eq!(status.attempts_remaining, 0);
    assert!(status.locked_seconds_remaining > 0);
    assert!(status.locked_seconds_remaining <= PIN_LOCKOUT_SECONDS);

    // Any attempt while locked returns remaining lock time
    let status_locked = guard.verify("123456", Some(&hash)); // Even valid PIN fails
    assert!(!status_locked.valid);
    assert_eq!(status_locked.attempts_remaining, 0);
    assert!(status_locked.locked_seconds_remaining > 0);
}
