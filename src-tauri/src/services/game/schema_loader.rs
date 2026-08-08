use serde::{Deserialize, Serialize};

use crate::domain::errors::AppError;
use crate::domain::models::GameType;

/// Game schema defines available categories and filter fields per game type.
/// Loaded from bundled JSON resources, with fallback to defaults.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GameSchema {
    pub categories: Vec<CategoryDef>,
    pub filters: Vec<FilterDef>,
    /// Optional stopwords to exclude during tokenization.
    #[serde(default)]
    pub stopwords: Vec<String>,
    /// Optional short token whitelist (tokens normally filtered but allowed).
    #[serde(default)]
    pub short_token_whitelist: Vec<String>,
    /// Optional INI key blacklist (keys to skip during extraction).
    #[serde(default)]
    pub ini_key_blacklist: Vec<String>,
    /// Optional INI key whitelist (keys to include during extraction).
    #[serde(default)]
    pub ini_key_whitelist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CategoryDef {
    pub name: String,
    /// Display label for the category. Falls back to `name` if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub icon: String,
    pub color: String,
    /// Per-category metadata filter fields. If absent, no metadata editing for this category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<FilterDef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct FilterDef {
    pub key: String,
    pub label: String,
    pub options: Vec<String>,
}

/// Categories the fallback schema offers, as `(name, icon, color)`.
/// Per TRD: fall back to [Character, Weapon, UI, Other].
const DEFAULT_CATEGORIES: [(&str, &str, &str); 4] = [
    ("Character", "User", "primary"),
    ("Weapon", "Sword", "secondary"),
    ("UI", "Layout", "accent"),
    ("Other", "Package", "neutral"),
];

/// Default schema fallback when game-specific schema.json is missing/corrupt.
pub fn default_schema() -> GameSchema {
    GameSchema {
        categories: DEFAULT_CATEGORIES
            .iter()
            .map(|(name, icon, color)| CategoryDef {
                name: (*name).to_string(),
                label: None,
                icon: (*icon).to_string(),
                color: (*color).to_string(),
                filters: None,
            })
            .collect(),
        filters: Vec::new(),
        stopwords: Vec::new(),
        short_token_whitelist: Vec::new(),
        ini_key_blacklist: Vec::new(),
        ini_key_whitelist: Vec::new(),
    }
}

/// Map a `GameType` discriminant to its canonical XXMI resource code.
/// The value is the `Serialize_repr` discriminant the frontend round-trips;
/// an unrecognised one falls back to gimi.
pub fn normalize_game_type(raw: i32) -> String {
    GameType::from_repr(raw)
        .unwrap_or(GameType::GIMI)
        .resource_code()
        .to_string()
}

/// Load a game schema from the bundled resources directory.
/// Falls back to `default_schema()` if the file is missing or corrupt.
///
/// # Arguments
/// * `resource_dir` - Base path to the app's resources directory
/// * `game_type` - `GameType` discriminant (GIMI=0 … EFMI=4)
pub fn load_schema(resource_dir: &std::path::Path, game_type: i32) -> GameSchema {
    read_schema(resource_dir, game_type).unwrap_or_else(|reason| {
        log::warn!("Schema unavailable for game type {game_type}: {reason}. Using fallback.");
        default_schema()
    })
}

/// Every failure mode reports why and lets the caller apply the one fallback.
fn read_schema(resource_dir: &std::path::Path, game_type: i32) -> Result<GameSchema, AppError> {
    let canonical = normalize_game_type(game_type);
    let schema_path = resource_dir
        .join("schemas")
        .join(format!("{}.json", canonical));

    log::info!(
        "Loading schema for game type {} (canonical: '{}') from: {}",
        game_type,
        canonical,
        schema_path.display()
    );

    let contents = std::fs::read_to_string(&schema_path)?;
    let schema: GameSchema = serde_json::from_str(&contents)?;

    if schema.categories.is_empty() {
        return Err(AppError::Internal(
            "schema has empty categories".to_string(),
        ));
    }

    Ok(schema)
}

#[cfg(test)]
#[path = "tests/schema_loader_tests.rs"]
mod tests;
