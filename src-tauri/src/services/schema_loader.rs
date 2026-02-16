use serde::{Deserialize, Serialize};

/// Game schema defines available categories and filter fields per game type.
/// Loaded from bundled JSON resources, with fallback to defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSchema {
    pub categories: Vec<CategoryDef>,
    pub filters: Vec<FilterDef>,
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
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // Covers: NC-3.4-02 (Schema Load Failure → fallback)
    #[test]
    fn test_default_schema_has_four_categories() {
        let schema = default_schema();
        assert_eq!(schema.categories.len(), 4);
        assert_eq!(schema.categories[0].name, "Character");
        assert_eq!(schema.categories[1].name, "Weapon");
        assert_eq!(schema.categories[2].name, "UI");
        assert_eq!(schema.categories[3].name, "Other");
        assert!(schema.filters.is_empty());
    }

    // Covers: NC-3.4-02 (Schema file missing → fallback)
    #[test]
    fn test_load_schema_missing_file_returns_default() {
        let temp = TempDir::new().unwrap();
        let schema = load_schema(temp.path(), "GIMI");
        assert_eq!(schema.categories.len(), 4);
        assert_eq!(schema.categories[0].name, "Character");
    }

    // Covers: NC-3.4-02 (Schema corrupt → fallback)
    #[test]
    fn test_load_schema_corrupt_json_returns_default() {
        let temp = TempDir::new().unwrap();
        let schemas_dir = temp.path().join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();
        let mut file = std::fs::File::create(schemas_dir.join("gimi.json")).unwrap();
        file.write_all(b"{ invalid json !!!").unwrap();

        let schema = load_schema(temp.path(), "GIMI");
        assert_eq!(schema.categories.len(), 4);
    }

    // Covers: DI-3.02 (Schema with empty categories → fallback)
    #[test]
    fn test_load_schema_empty_categories_returns_default() {
        let temp = TempDir::new().unwrap();
        let schemas_dir = temp.path().join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();
        let mut file = std::fs::File::create(schemas_dir.join("gimi.json")).unwrap();
        file.write_all(b"{\"categories\": [], \"filters\": []}")
            .unwrap();

        let schema = load_schema(temp.path(), "GIMI");
        assert_eq!(schema.categories.len(), 4, "Should fallback on empty");
    }

    // Covers: TC-3.4 (Valid schema loads correctly)
    #[test]
    fn test_load_schema_valid_json_returns_parsed() {
        let temp = TempDir::new().unwrap();
        let schemas_dir = temp.path().join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();

        let valid_json = r#"{
            "categories": [
                { "name": "Resonator", "icon": "User", "color": "primary" },
                { "name": "Weapon", "icon": "Sword", "color": "secondary" }
            ],
            "filters": [
                { "key": "element", "label": "Element", "options": ["Spectro", "Havoc"] }
            ]
        }"#;

        let mut file = std::fs::File::create(schemas_dir.join("wwmi.json")).unwrap();
        file.write_all(valid_json.as_bytes()).unwrap();

        let schema = load_schema(temp.path(), "WWMI");
        assert_eq!(schema.categories.len(), 2);
        assert_eq!(schema.categories[0].name, "Resonator");
        assert_eq!(schema.filters.len(), 1);
        assert_eq!(schema.filters[0].key, "element");
        assert_eq!(schema.filters[0].options.len(), 2);
    }

    // Covers: TC-3.1-01 (case insensitive file lookup)
    #[test]
    fn test_load_schema_case_insensitive_game_type() {
        let temp = TempDir::new().unwrap();
        let schemas_dir = temp.path().join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();

        let valid =
            r#"{"categories": [{"name": "Test", "icon": "X", "color": "info"}], "filters": []}"#;
        let mut file = std::fs::File::create(schemas_dir.join("srmi.json")).unwrap();
        file.write_all(valid.as_bytes()).unwrap();

        // Should find srmi.json even when called with "SRMI"
        let schema = load_schema(temp.path(), "SRMI");
        assert_eq!(schema.categories.len(), 1);
        assert_eq!(schema.categories[0].name, "Test");
    }

    // Covers: normalize_game_type maps legacy names to canonical XXMI codes
    #[test]
    fn test_normalize_game_type() {
        assert_eq!(normalize_game_type("StarRail"), "srmi");
        assert_eq!(normalize_game_type("SRMI"), "srmi");
        assert_eq!(normalize_game_type("Genshin"), "gimi");
        assert_eq!(normalize_game_type("GIMI"), "gimi");
        assert_eq!(normalize_game_type("ZZZ"), "zzmi");
        assert_eq!(normalize_game_type("Wuthering"), "wwmi");
        assert_eq!(normalize_game_type("Endfield"), "efmi");
        // Unknown passthrough
        assert_eq!(normalize_game_type("CustomGame"), "customgame");
    }

    // Covers: load_schema with legacy game_type resolves to correct schema
    #[test]
    fn test_load_schema_with_legacy_game_type() {
        let temp = TempDir::new().unwrap();
        let schemas_dir = temp.path().join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();

        let valid =
            r#"{"categories": [{"name": "Character", "icon": "User", "color": "primary", "filters": [{"key": "element", "label": "Element", "options": ["Fire"]}]}], "filters": []}"#;
        let mut file = std::fs::File::create(schemas_dir.join("srmi.json")).unwrap();
        file.write_all(valid.as_bytes()).unwrap();

        // "StarRail" (legacy) should normalize to "srmi" and find srmi.json
        let schema = load_schema(temp.path(), "StarRail");
        assert_eq!(schema.categories.len(), 1);
        assert_eq!(schema.categories[0].name, "Character");
        // Per-category filters should be present
        let filters = schema.categories[0].filters.as_ref().unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].key, "element");
    }
}
