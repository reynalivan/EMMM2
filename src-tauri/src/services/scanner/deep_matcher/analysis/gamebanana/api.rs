//! Blocking, fail-safe GameBanana API client.

use crate::common::sync::lock;
use crate::domain::errors::ScannerError;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use super::types::{GameBananaConfig, GameBananaGame, GameBananaRef, GameBananaResult};

const API_TIMEOUT_SECS: u64 = 5;
const RATE_LIMIT_MS: u64 = 1000;

// Store the fetched token so we don't spam the Auth endpoint
static GB_AUTH_TOKEN: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static GB_AUTH_FAILED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

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
                        let tokens = crate::common::normalizer::preprocess_text(&clean_desc);
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
                                    crate::common::normalizer::normalize_for_matching_default(
                                        &stem,
                                    );
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
                                    crate::common::normalizer::normalize_for_matching_default(
                                        &stem,
                                    );
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
    if *lock(&GB_AUTH_FAILED) {
        return None;
    }

    {
        let lock = lock(&GB_AUTH_TOKEN);
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
        *lock(&GB_AUTH_FAILED) = true;
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
                *lock(&GB_AUTH_TOKEN) = Some(token_str.clone());
                log::info!("GB: Successfully authenticated as app {}", app_id);
                Some(token_str)
            } else {
                log::warn!("GB: Authentication failed, token not found in response.");
                *lock(&GB_AUTH_FAILED) = true;
                None
            }
        }
        Err(e) => {
            log::warn!("GB: Authentication request failed: {e}");
            *lock(&GB_AUTH_FAILED) = true;
            None
        }
    }
}

fn fetch_json_value(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<serde_json::Value, ScannerError> {
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(ScannerError::Validation(format!(
            "HTTP {}",
            response.status()
        )));
    }

    Ok(response.json::<serde_json::Value>()?)
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
