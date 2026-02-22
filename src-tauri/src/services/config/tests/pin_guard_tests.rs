use super::*;

#[test]
fn test_validate_pin_format() {
    assert!(validate_pin_format("123456").is_ok());
    assert!(validate_pin_format("12345").is_err());
    assert!(validate_pin_format("1234ab").is_err());
}
