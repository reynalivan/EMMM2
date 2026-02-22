use serde::{Deserialize, Serialize};

/// Game schema defines available categories and filter fields per game type.
/// Loaded from bundled JSON resources, with fallback to defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDef {
    pub key: String,
    pub label: String,
    pub options: Vec<String>,
}

/// Default schema fallback when game-specific schema.json is missing/corrupt.
/// Per TRD: fall back to [Character, Weapon, UI, Other]. Log WARN.
pub fn default_schema() -> GameSchema {
    GameSchema {
        categories: vec![
            CategoryDef {
                name: "Character".to_string(),
                label: None,
                icon: "User".to_string(),
                color: "primary".to_string(),
                filters: None,
            },
            CategoryDef {
                name: "Weapon".to_string(),
                label: None,
                icon: "Sword".to_string(),
                color: "secondary".to_string(),
                filters: None,
            },
            CategoryDef {
                name: "UI".to_string(),
                label: None,
                icon: "Layout".to_string(),
                color: "accent".to_string(),
                filters: None,
            },
            CategoryDef {
                name: "Other".to_string(),
                label: None,
                icon: "Package".to_string(),
                color: "neutral".to_string(),
                filters: None,
            },
        ],
        filters: vec![],
        stopwords: vec![],
        short_token_whitelist: vec![],
        ini_key_blacklist: vec![],
        ini_key_whitelist: vec![],
    }
}

/// Normalize legacy game_type values to canonical XXMI codes.
/// Maps alternative names (e.g. "StarRail" → "srmi", "Genshin" → "gimi") so that
/// resource lookups (schemas, databases, thumbnails) resolve correctly.
pub fn normalize_game_type(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "genshin" | "genshinimpact" | "genshin_impact" | "gimi" => "gimi".to_string(),
        "starrail" | "star_rail" | "honkaistarrail" | "hsr" | "srmi" => "srmi".to_string(),
        "zzz" | "zenless" | "zenlesszonezero" | "zzmi" => "zzmi".to_string(),
        "wuthering" | "wutheringwaves" | "wuwa" | "wwmi" => "wwmi".to_string(),
        "endfield" | "arknightendfield" | "arknight" | "efmi" => "efmi".to_string(),
        other => other.to_string(),
    }
}

/// Load a game schema from the bundled resources directory.
/// Falls back to `default_schema()` if the file is missing or corrupt.
///
/// # Arguments
/// * `resource_dir` - Base path to the app's resources directory
/// * `game_type` - Game type string (e.g., "GIMI", "SRMI", or legacy "StarRail")
pub fn load_schema(resource_dir: &std::path::Path, game_type: &str) -> GameSchema {
    let canonical = normalize_game_type(game_type);
    let schema_path = resource_dir
        .join("schemas")
        .join(format!("{}.json", canonical));

    log::info!(
        "Loading schema for '{}' (canonical: '{}') from: {}",
        game_type,
        canonical,
        schema_path.display()
    );

    match std::fs::read_to_string(&schema_path) {
        Ok(contents) => match serde_json::from_str::<GameSchema>(&contents) {
            Ok(schema) => {
                if schema.categories.is_empty() {
                    log::warn!(
                        "Schema for {} has empty categories, using fallback",
                        game_type
                    );
                    default_schema()
                } else {
                    schema
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to parse schema for {}: {}. Using fallback.",
                    game_type,
                    e
                );
                default_schema()
            }
        },
        Err(e) => {
            log::warn!(
                "Schema file not found for {}: {}. Using fallback.",
                game_type,
                e
            );
            default_schema()
        }
    }
}

#[cfg(test)]
#[path = "tests/schema_loader_tests.rs"]
mod tests;
