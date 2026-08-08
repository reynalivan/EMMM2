use super::*;

// TC-30-011: Validating the PIN format
#[test]
fn test_validate_pin_format() {
    assert!(validate_pin_format("123456").is_ok());
    assert!(validate_pin_format("12345").is_err()); // Too short
    assert!(validate_pin_format("1234567").is_err()); // Too long
    assert!(validate_pin_format("1234ab").is_err()); // Non-numeric
}
