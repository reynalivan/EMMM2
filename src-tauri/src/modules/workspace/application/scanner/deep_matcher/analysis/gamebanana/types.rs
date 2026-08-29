//! Public data types for GameBanana enrichment.

// ── Game-Specific IDs ────────────────────────────────────────────────

/// Known GameBanana game IDs for supported mod loaders.
///
/// Used to scope API queries by game when the caller knows which game
/// the mod folder belongs to, improving enrichment relevance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameBananaGame {
    /// GIMI — Genshin Impact
    Genshin,
    /// SRMI — Honkai: Star Rail
    StarRail,
    /// ZZMI — Zenless Zone Zero
    ZenlessZoneZero,
    /// WWMI — Wuthering Waves
    WutheringWaves,
    /// EFMI — Arknights: Endfield
    ArknightsEndfield,
}

impl GameBananaGame {
    /// Returns the numeric GameBanana game ID.
    pub fn game_id(self) -> u64 {
        match self {
            Self::Genshin => 8552,
            Self::StarRail => 18366,
            Self::ZenlessZoneZero => 19567,
            Self::WutheringWaves => 20357,
            Self::ArknightsEndfield => 21842,
        }
    }

    /// Returns the GameBanana game slug for URL construction.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Genshin => "genshin-impact",
            Self::StarRail => "honkai-star-rail",
            Self::ZenlessZoneZero => "zenless-zone-zero",
            Self::WutheringWaves => "wuthering-waves",
            Self::ArknightsEndfield => "arknights-endfield",
        }
    }

    /// Returns the GameBanana game name string.
    pub fn name(self) -> &'static str {
        match self {
            Self::Genshin => "Genshin Impact",
            Self::StarRail => "Honkai: Star Rail",
            Self::ZenlessZoneZero => "Zenless Zone Zero",
            Self::WutheringWaves => "Wuthering Waves",
            Self::ArknightsEndfield => "Arknights: Endfield",
        }
    }
}

// ── Types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameBananaRef {
    pub item_type: String,
    pub item_id: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GameBananaConfig {
    pub enabled: bool,
    /// When set, scopes API queries to this game's GameBanana section.
    pub game: Option<GameBananaGame>,
}

#[derive(Debug, Clone, Default)]
pub struct GameBananaResult {
    /// Normalized file stems from the mod's _aFiles (e.g. "ultimate_dimentio").
    pub file_stems: Vec<String>,
    /// Optional mod name from the API.
    pub mod_name: Option<String>,
    /// Root category name (e.g. "Skins", "Other/Misc") for type validation.
    pub root_category: Option<String>,
    /// Keywords extracted from the description.
    pub description_keywords: Vec<String>,
}
