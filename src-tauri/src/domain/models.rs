use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt;
use std::str::FromStr;

/// Supported game types (3DMigoto modding frameworks)
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq, sqlx::Type)]
#[repr(u8)]
pub enum GameType {
    GIMI = 0, // Genshin Impact
    SRMI = 1, // Honkai Star Rail
    WWMI = 2, // Wuthering Waves
    ZZMI = 3, // Zenless Zone Zero
    EFMI = 4, // Arknight Endfield
}

// serde_repr serializes this enum as its numeric discriminant, but the specta
// derive would export the variant *names* ("GIMI" | ...) — a wire-format lie.
// Delegate to u8 so the generated TS sees `number`, matching serde_repr.
impl specta::Type for GameType {
    fn inline(
        type_map: &mut specta::TypeCollection,
        generics: specta::Generics,
    ) -> specta::DataType {
        <u8 as specta::Type>::inline(type_map, generics)
    }
}

/// Status of an object or mod
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq, sqlx::Type)]
#[repr(i64)]
#[derive(Default)]
pub enum ItemStatus {
    Disabled = 0,
    #[default]
    Enabled = 1,
}

// Same serde_repr/specta mismatch as GameType above: export as `number`.
impl specta::Type for ItemStatus {
    fn inline(
        type_map: &mut specta::TypeCollection,
        generics: specta::Generics,
    ) -> specta::DataType {
        <i64 as specta::Type>::inline(type_map, generics)
    }
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
    /// Every supported game, in discriminant order. The single roster — callers
    /// that need "all games" must iterate this rather than restate the list.
    pub const ALL: [GameType; 5] = [
        GameType::GIMI,
        GameType::SRMI,
        GameType::WWMI,
        GameType::ZZMI,
        GameType::EFMI,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            GameType::GIMI => "Genshin Impact",
            GameType::SRMI => "Honkai Star Rail",
            GameType::WWMI => "Wuthering Waves",
            GameType::ZZMI => "Zenless Zone Zero",
            GameType::EFMI => "Arknight Endfield",
        }
    }

    /// The lowercase code used to name bundled resource files (schemas, MasterDB).
    pub fn resource_code(&self) -> &'static str {
        match self {
            GameType::GIMI => "gimi",
            GameType::SRMI => "srmi",
            GameType::WWMI => "wwmi",
            GameType::ZZMI => "zzmi",
            GameType::EFMI => "efmi",
        }
    }

    /// Recover a `GameType` from the `Serialize_repr` discriminant the frontend
    /// round-trips. Returns `None` for a value outside the enum.
    pub fn from_repr(raw: i32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|game_type| *game_type as i32 == raw)
    }
}

/// Result of a successful folder validation
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameInfo {
    pub path: String,
    pub launcher_path: String,
    pub mods_path: String,
}

/// Startup config status returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub enum ConfigStatus {
    FreshInstall,
    HasConfig,
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

/// Represents a row in the `objects` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct GameObject {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub status: ItemStatus,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub tags: String,
    pub metadata: String,
    pub hash_db: Option<HashDbPayload>,
    pub custom_skins: Option<CustomSkinsPayload>,
    pub thumbnail_path: Option<String>,
    pub is_pinned: bool,
    pub is_auto_sync: bool,
    pub created_at: String,
}
