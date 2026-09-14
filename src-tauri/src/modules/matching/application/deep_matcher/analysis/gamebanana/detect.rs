//! Detection of `gamebanana.com/<type>/<id>` references in folder signals.

use regex::Regex;
use std::sync::LazyLock;

use crate::modules::matching::application::deep_matcher::analysis::content::FolderSignals;

use super::types::GameBananaRef;

static GB_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"gamebanana\.com/(mods|tools|scripts|skins)/(\d+)").unwrap());

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

/// Extract a single submission reference from a trusted browser page URL.
pub fn gamebanana_reference_from_url(url: &str) -> Option<GameBananaRef> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }

    let host = parsed.host_str()?.to_ascii_lowercase();
    if host != "gamebanana.com" && host != "www.gamebanana.com" {
        return None;
    }

    let mut segments = parsed.path().trim_matches('/').split('/');
    let item_type = segments.next()?;
    let item_id = segments.next()?.parse::<u64>().ok()?;
    if segments.next().is_some() || !matches!(item_type, "mods" | "tools" | "scripts" | "skins") {
        return None;
    }

    Some(GameBananaRef {
        item_type: capitalize_type(item_type),
        item_id,
    })
}

fn capitalize_type(raw: &str) -> String {
    // GameBanana web URLs use plural (mods/skins), API uses singular (Mod/Skin)
    let base = if let Some(stripped) = raw.strip_suffix('s') {
        stripped
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
