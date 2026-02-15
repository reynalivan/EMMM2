use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported game types (3DMigoto modding frameworks)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameType {
    GIMI, // Genshin Impact
    SRMI, // Honkai Star Rail
    WWMI, // Wuthering Waves
    ZZMI, // Zenless Zone Zero
    EFMI, // Arknight Endfield
}

impl fmt::Display for GameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameType::GIMI => write!(f, "GIMI"),
            GameType::SRMI => write!(f, "SRMI"),
            GameType::WWMI => write!(f, "WWMI"),
            GameType::ZZMI => write!(f, "ZZMI"),
            GameType::EFMI => write!(f, "EFMI"),
        }
    }
}

impl FromStr for GameType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GIMI" => Ok(GameType::GIMI),
            "SRMI" => Ok(GameType::SRMI),
            "WWMI" => Ok(GameType::WWMI),
            "ZZMI" => Ok(GameType::ZZMI),
            "EFMI" => Ok(GameType::EFMI),
            _ => Err(format!("Unknown game type: {s}")),
        }
    }
}

impl GameType {
    pub fn display_name(&self) -> &'static str {
        match self {
            GameType::GIMI => "Genshin Impact",
            GameType::SRMI => "Honkai Star Rail",
            GameType::WWMI => "Wuthering Waves",
            GameType::ZZMI => "Zenless Zone Zero",
            GameType::EFMI => "Arknight Endfield",
        }
    }
}

/// Result of a successful folder validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub path: String,
    pub launcher_path: String,
    pub mods_path: String,
}

/// Full game configuration stored in DB + config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: String,
    pub path: String,
    pub mods_path: String,
    pub launcher_path: String,
    pub launch_args: Option<String>,
}

/// Application-wide error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Duplicate: {0}")]
    Duplicate(String),

    #[error("Config error: {0}")]
    Config(String),
}

// Tauri commands need Serialize for error types
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Startup config status returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigStatus {
    FreshInstall,
    HasConfig,
    CorruptConfig,
}

#[cfg(test)]
mod tests {
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
}
