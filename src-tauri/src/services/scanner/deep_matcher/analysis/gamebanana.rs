//! GameBanana metadata enrichment for the deep matcher pipeline.
//!
//! Detects GameBanana URLs in INI data/signals, fetches file metadata via their
//! public API, and returns normalized file stems as enrichment data.
//!
//! **Fail-safe**: All network failures are logged and skipped. This module
//! never blocks the pipeline.

use regex::Regex;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;

static GB_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"gamebanana\.com/(mods|tools|scripts|skins)/(\d+)").unwrap());

const API_TIMEOUT_SECS: u64 = 5;
const RATE_LIMIT_MS: u64 = 1000;

// Store the fetched token so we don't spam the Auth endpoint
static GB_AUTH_TOKEN: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static GB_AUTH_FAILED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

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

    /// Attempt to resolve from a game key string (e.g. "gimi", "srmi").
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_lowercase().as_str() {
            "gimi" | "genshin" => Some(Self::Genshin),
            "srmi" | "starrail" | "star_rail" | "hsr" => Some(Self::StarRail),
            "zzmi" | "zzz" | "zenless" => Some(Self::ZenlessZoneZero),
            "wwmi" | "wuwa" | "wuthering" => Some(Self::WutheringWaves),
            "efmi" | "endfield" | "arknights" => Some(Self::ArknightsEndfield),
            _ => None,
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

// ── URL Detection ────────────────────────────────────────────────────

/// Scan all text signals for `gamebanana.com/<type>/<id>` patterns.
///
/// Returns deduplicated refs. Scans ini_content_tokens, deep_name_strings,
/// and folder_tokens.
pub fn detect_gamebanana_ids(signals: &FolderSignals) -> Vec<GameBananaRef> {
    let mut seen = std::collections::HashSet::new();
    let mut refs = Vec::new();

    let all_strings = signals
        .ini_content_tokens
        .iter()
        .chain(signals.deep_name_strings.iter())
        .chain(signals.folder_tokens.iter())
        .chain(signals.ini_derived_strings.iter());

    for text in all_strings {
        for capture in GB_URL_REGEX.captures_iter(text) {
            let item_type = capitalize_type(&capture[1]);
            let Ok(item_id) = capture[2].parse::<u64>() else {
                continue;
            };

            let key = (item_type.clone(), item_id);
            if seen.insert(key) {
                refs.push(GameBananaRef { item_type, item_id });
            }
        }
    }

    refs
}

// ── API Client (blocking, fail-safe) ─────────────────────────────────

/// Fetch file names and optional mod name from GameBanana API.
///
/// Uses `reqwest::blocking::Client`. Returns `GameBananaResult::default()` on any
/// failure. Rate-limits at 1 request/second between sequential calls.
///
/// When `config.game` is set, uses it to validate that the mod belongs to
/// the expected game via the secondary Core/Item/Data endpoint.
pub fn fetch_gamebanana_metadata(
    refs: &[GameBananaRef],
    config: &GameBananaConfig,
) -> GameBananaResult {
    if refs.is_empty() {
        return GameBananaResult::default();
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("GB: failed to build HTTP client: {e}");
            return GameBananaResult::default();
        }
    };

    let mut all_stems = Vec::new();
    let mut mod_name: Option<String> = None;
    let mut root_category: Option<String> = None;
    let mut description_keywords = Vec::new();

    for (i, gb_ref) in refs.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(Duration::from_millis(RATE_LIMIT_MS));
        }

        // Optional: validate game ownership via Core/Item/Data
        if let Some(game) = config.game {
            if !validate_game_ownership(&client, gb_ref, game) {
                log::debug!(
                    "GB: skipping {}/{} — does not belong to game {}",
                    gb_ref.item_type,
                    gb_ref.item_id,
                    game.slug(),
                );
                continue;
            }
        }

        // Primary: fetch file list via v11 endpoint
        let mut file_list_url = format!(
            "https://api.gamebanana.com/Core/Item/Data?itemtype={}&itemid={}&fields=name,Files().aFiles(),RootCategory().name,description&return_keys=1",
            gb_ref.item_type, gb_ref.item_id
        );

        if let Some(token) = get_gb_auth_token(&client) {
            file_list_url.push_str(&format!("&_sToken={}", token));
        }

        match fetch_json_value(&client, &file_list_url) {
            Ok(json) => {
                // Extract mod name
                if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                    if mod_name.is_none() {
                        mod_name = Some(name.to_string());
                    }
                }

                // Extract root category
                if let Some(cat) = json.get("RootCategory().name").and_then(|v| v.as_str()) {
                    if root_category.is_none() {
                        root_category = Some(cat.to_string());
                    }
                }

                // Extract description and tokenize
                if let Some(desc) = json.get("description").and_then(|v| v.as_str()) {
                    if description_keywords.is_empty() {
                        // Strip basic HTML tags before tokenizing
                        let clean_desc = strip_html_tags(desc);
                        let tokens = crate::services::scanner::core::normalizer::preprocess_text(
                            &clean_desc,
                        );
                        description_keywords.extend(tokens.into_iter().filter(|w| w.len() >= 3));
                    }
                }

                // Extract file stems from Files().aFiles()
                // GameBanana API returns this field as an object map, e.g. {"1234": {"_sFile": "...", ...}}
                // or optionally as an array if sparsely serialized, so we handle both cleanly.
                if let Some(files_map) = json.get("Files().aFiles()") {
                    if let Some(obj) = files_map.as_object() {
                        for (_id, file_obj) in obj {
                            if let Some(filename) = file_obj.get("_sFile").and_then(|v| v.as_str())
                            {
                                let stem = strip_extension(filename);
                                let normalized =
                                    crate::services::scanner::core::normalizer::normalize_for_matching_default(&stem);
                                if !normalized.is_empty() {
                                    all_stems.push(normalized);
                                }
                            }
                        }
                    } else if let Some(arr) = files_map.as_array() {
                        for file_obj in arr {
                            if let Some(filename) = file_obj.get("_sFile").and_then(|v| v.as_str())
                            {
                                let stem = strip_extension(filename);
                                let normalized =
                                    crate::services::scanner::core::normalizer::normalize_for_matching_default(&stem);
                                if !normalized.is_empty() {
                                    all_stems.push(normalized);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "GB: v11 data fetch failed for {}/{}: {e}",
                    gb_ref.item_type,
                    gb_ref.item_id,
                );
            }
        }
    }

    // Deduplicate description keywords
    description_keywords.sort();
    description_keywords.dedup();

    GameBananaResult {
        file_stems: all_stems,
        mod_name,
        root_category,
        description_keywords,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Validate that a mod belongs to the expected game via Core/Item/Data.
///
/// Returns `true` on success or on any failure (fail-open: don't block
/// enrichment if validation itself fails).
fn validate_game_ownership(
    client: &reqwest::blocking::Client,
    gb_ref: &GameBananaRef,
    expected_game: GameBananaGame,
) -> bool {
    let mut url = format!(
        "https://api.gamebanana.com/Core/Item/Data?itemtype={}&itemid={}&fields=Game().name&return_keys=1",
        gb_ref.item_type, gb_ref.item_id
    );

    if let Some(token) = get_gb_auth_token(client) {
        url.push_str(&format!("&_sToken={}", token));
    }

    match fetch_json_value(client, &url) {
        Ok(json) => {
            // Check Game().name matches expected game name
            let actual_game_name = json.get("Game().name").and_then(|name| name.as_str());

            match actual_game_name {
                Some(name) => {
                    if name == expected_game.name() {
                        true
                    } else {
                        log::debug!(
                            "GB: validation failed, name mismatch. expected: {}, got: {}",
                            expected_game.name(),
                            name
                        );
                        false
                    }
                }
                None => {
                    log::debug!("GB: validation failed-open, could not parse Game().name");
                    true // fail-open if field missing
                }
            }
        }
        Err(e) => {
            log::debug!("GB: game validation failed (fail-open): {e}");
            true // fail-open
        }
    }
}

fn get_gb_auth_token(client: &reqwest::blocking::Client) -> Option<String> {
    if *GB_AUTH_FAILED.lock().unwrap() {
        return None;
    }

    {
        let lock = GB_AUTH_TOKEN.lock().unwrap();
        if let Some(token) = lock.as_ref() {
            return Some(token.clone());
        }
    }

    let _ = dotenvy::dotenv(); // Try to load .env, ignore if missing

    let app_id = std::env::var("GB_APP_ID").unwrap_or_default();
    let user_id = std::env::var("GB_USER_ID").unwrap_or_default();
    let api_password = std::env::var("GB_API_PASSWORD").unwrap_or_default();

    if app_id.is_empty() || user_id.is_empty() || api_password.is_empty() {
        log::debug!("GB: Auth credentials missing in .env, falling back to public API.");
        *GB_AUTH_FAILED.lock().unwrap() = true;
        return None;
    }

    let url = format!(
        "https://api.gamebanana.com/Core/App/Authenticate?app_id={}&userid={}&api_password={}",
        app_id,
        user_id,
        urlencoding::encode(&api_password)
    );

    match fetch_json_value(client, &url) {
        Ok(json) => {
            if let Some(token) = json.get("_sToken").and_then(|v| v.as_str()) {
                let token_str = token.to_string();
                *GB_AUTH_TOKEN.lock().unwrap() = Some(token_str.clone());
                log::info!("GB: Successfully authenticated as app {}", app_id);
                Some(token_str)
            } else {
                log::warn!("GB: Authentication failed, token not found in response.");
                *GB_AUTH_FAILED.lock().unwrap() = true;
                None
            }
        }
        Err(e) => {
            log::warn!("GB: Authentication request failed: {e}");
            *GB_AUTH_FAILED.lock().unwrap() = true;
            None
        }
    }
}

fn fetch_json_value(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<serde_json::Value, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<serde_json::Value>()
        .map_err(|e| format!("JSON parse failed: {e}"))
}

fn strip_extension(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((stem, _ext)) => stem.to_string(),
        None => filename.to_string(),
    }
}

// Simple HTML tag stripper since GameBanana descriptions are rich text
fn strip_html_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(c);
        }
    }

    // Convert minimal entities like &nbsp; to space (more complex ones handled by normalizer usually)
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn capitalize_type(raw: &str) -> String {
    // GameBanana web URLs use plural (mods/skins), API uses singular (Mod/Skin)
    let base = if raw.ends_with('s') {
        &raw[..raw.len() - 1]
    } else {
        raw
    };

    let mut chars = base.chars();
    match chars.next() {
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            upper + chars.as_str()
        }
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "../tests/analysis/gamebanana_tests.rs"]
mod tests;
