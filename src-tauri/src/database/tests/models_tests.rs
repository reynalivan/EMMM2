use super::*;

#[test]
fn test_game_type_from_str_valid() {
    assert_eq!("GIMI".parse::<GameType>().unwrap(), GameType::GIMI);
    assert_eq!("srmi".parse::<GameType>().unwrap(), GameType::SRMI);
    assert_eq!("Wwmi".parse::<GameType>().unwrap(), GameType::WWMI);
}

#[test]
fn test_game_type_from_str_invalid() {
    assert!("INVALID".parse::<GameType>().is_err());
    assert!("".parse::<GameType>().is_err());
}

#[test]
fn test_game_type_display() {
    assert_eq!(GameType::GIMI.to_string(), "GIMI");
    assert_eq!(GameType::ZZMI.display_name(), "Zenless Zone Zero");
}

#[test]
fn test_app_error_display() {
    let err = AppError::Validation("test".to_string());
    assert_eq!(err.to_string(), "Validation failed: test");
}
