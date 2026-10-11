use super::*;

#[test]
fn game_type_numeric_and_label_wire_contracts() {
    for (game_type, discriminant, label, resource) in [
        (GameType::Gimi, 0, "GIMI", "gimi"),
        (GameType::Srmi, 1, "SRMI", "srmi"),
        (GameType::Wwmi, 2, "WWMI", "wwmi"),
        (GameType::Zzmi, 3, "ZZMI", "zzmi"),
        (GameType::Efmi, 4, "EFMI", "efmi"),
    ] {
        let encoded = serde_json::to_value(game_type).unwrap();
        assert_eq!(encoded, serde_json::json!(discriminant));
        assert_eq!(
            serde_json::from_value::<GameType>(encoded).unwrap(),
            game_type
        );
        assert_eq!(GameType::from_repr(discriminant), Some(game_type));
        assert_eq!(game_type.to_string(), label);
        assert_eq!(game_type.display_name(), label);
        assert_eq!(label.parse::<GameType>().unwrap(), game_type);
        assert_eq!(resource.parse::<GameType>().unwrap(), game_type);
        assert_eq!(game_type.resource_code(), resource);
    }
    assert!(serde_json::from_str::<GameType>("5").is_err());
}

#[test]
fn test_game_type_from_str_valid() {
    assert_eq!("GIMI".parse::<GameType>().unwrap(), GameType::Gimi);
    assert_eq!("srmi".parse::<GameType>().unwrap(), GameType::Srmi);
    assert_eq!("Wwmi".parse::<GameType>().unwrap(), GameType::Wwmi);
}

#[test]
fn test_game_type_from_str_invalid() {
    assert!("INVALID".parse::<GameType>().is_err());
    assert!("".parse::<GameType>().is_err());
}

#[test]
fn test_game_type_display() {
    assert_eq!(GameType::Gimi.to_string(), "GIMI");
    assert_eq!(GameType::Zzmi.display_name(), "ZZMI");
}

use crate::shared::errors::AppError;

#[test]
fn test_app_error_display() {
    let err = AppError::Internal("test".to_string());
    assert_eq!(err.to_string(), "Internal error: test");
}
