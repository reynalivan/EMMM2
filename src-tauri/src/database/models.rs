use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt;
use std::str::FromStr;

/// Supported game types (3DMigoto modding frameworks)
#[derive(
    Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq, specta::Type, sqlx::Type,
)]
#[repr(u8)]
pub enum GameType {
    GIMI = 0, // Genshin Impact
    SRMI = 1, // Honkai Star Rail
    WWMI = 2, // Wuthering Waves
    ZZMI = 3, // Zenless Zone Zero
    EFMI = 4, // Arknight Endfield
}

/// Status of an object or mod
#[derive(
    Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq, specta::Type, sqlx::Type,
)]
#[repr(i64)]
#[derive(Default)]
pub enum ItemStatus {
    Disabled = 0,
    #[default]
    Enabled = 1,
}

impl ItemStatus {
    pub fn is_enabled(&self) -> bool {
        *self == ItemStatus::Enabled
    }

    pub fn from_is_disabled(disabled: bool) -> Self {
        if disabled {
            ItemStatus::Disabled
        } else {
            ItemStatus::Enabled
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ItemStatus::Enabled => "active",
            ItemStatus::Disabled => "disabled",
        }
    }
}

impl FromStr for ItemStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" | "enabled" | "1" => Ok(ItemStatus::Enabled),
            "disabled" | "0" => Ok(ItemStatus::Disabled),
            _ => Err(format!("Unknown status: {s}")),
        }
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameInfo {
    pub path: String,
    pub launcher_path: String,
    pub mods_path: String,
}

/// Full game configuration stored in DB + config
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub game_type: GameType,
    pub path: String,
    pub mods_path: String,
    pub launcher_path: String,
    pub launch_args: Option<String>,
}

// (AppError removed because it moved to domain/errors.rs and was causing specta::Type issues with std::io::Error)

/// Startup config status returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub enum ConfigStatus {
    FreshInstall,
    HasConfig,
    CorruptConfig,
}

/// Strongly-typed payload for an object's known hashed files
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Default)]
pub struct HashDbPayload(pub std::collections::HashMap<String, Vec<String>>);

impl sqlx::Type<sqlx::Sqlite> for HashDbPayload {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for HashDbPayload {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let text = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Self::default());
        }
        let parsed: serde_json::Value = serde_json::from_str(trimmed)?;
        if parsed.is_array() {
            // Gracefully ignore array payloads from legacy bugs
            Ok(Self::default())
        } else {
            Ok(serde_json::from_value(parsed)?)
        }
    }
}

/// Strongly-typed payload for custom skins attached to the master DB
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Default)]
pub struct CustomSkinsPayload(pub std::collections::HashMap<String, String>);

impl sqlx::Type<sqlx::Sqlite> for CustomSkinsPayload {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for CustomSkinsPayload {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let text = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Self::default());
        }
        let parsed: serde_json::Value = serde_json::from_str(trimmed)?;
        if parsed.is_array() {
            // Gracefully ignore array payloads from legacy bugs
            Ok(Self::default())
        } else {
            Ok(serde_json::from_value(parsed)?)
        }
    }
}

#[cfg(test)]
#[path = "../repo/tests/models_test.rs"]
mod tests;
