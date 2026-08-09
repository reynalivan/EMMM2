//! Text normalization for mod folder/file names.
//! Handles transliteration, tokenization, and sanitization per TRD §3.2.

use std::borrow::Cow;

use deunicode::deunicode;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Compiled regex for stripping non-alphanumeric characters.
static RE_NON_ALNUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^a-zA-Z0-9\s]").expect("Invalid regex"));

/// Common noise prefixes to strip from folder names before matching.
const NOISE_PREFIXES: &[&str] = &["[mod]", "[skin]", "[fix]", "[update]"];

/// Preprocess text into a normalized token set.
///
/// Pipeline:
/// 1. Transliterate non-Latin characters (CJK → Latin) via deunicode
/// 2. Strip non-alphanumeric symbols (keep spaces)
/// 3. Lowercase and split into whitespace-separated tokens
///
/// # Covers: Epic 2 §B.1
pub fn preprocess_text(text: &str) -> HashSet<String> {
    let text_latin = deunicode(text);
    let text_clean = RE_NON_ALNUM.replace_all(&text_latin, " ");
    text_clean
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Remove common noise prefixes from a folder name for cleaner matching.
///
/// Strips prefixes like `[Mod]`, `DISABLED `, `[Skin]` etc.
pub fn strip_noise_prefixes(name: &str) -> String {
    let disabled_stripped = normalize_display_name(name);
    if disabled_stripped != name.trim() {
        return disabled_stripped.into_owned();
    }

    let mut result = name.trim().to_string();
    let lower = result.to_lowercase();

    for prefix in NOISE_PREFIXES {
        if lower.starts_with(prefix) {
            result = result[prefix.len()..].to_string();
            break;
        }
    }

    result.trim().to_string()
}

/// Leading word every DISABLED spelling shares, used as a cheap regex prefilter.
const DISABLED_WORD: &str = "disabled";

/// Regex matching canonical and legacy DISABLED folder prefixes.
static DISABLED_DETECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^disabled[\s_-]+").unwrap());

/// Normalize a folder name for UI display.
///
/// Strips canonical or legacy DISABLED prefix variants — repeatedly, so an
/// externally produced `DISABLED DISABLED Foo` still resolves to `Foo` — and
/// trims whitespace. This is the identity rule every path key derives from.
///
/// Always borrows. Most folder names are not prefixed, and this runs per path
/// component in the matcher, the explorer listing and every key comparison —
/// so the common case must not allocate or touch the regex.
pub fn normalize_display_name(name: &str) -> Cow<'_, str> {
    let mut value = name.trim();
    loop {
        // `DISABLED_DETECT_RE` is `(?i)^disabled[\s_-]+`; the cheap prefix
        // test rejects almost everything before the engine is entered.
        if !value
            .get(..DISABLED_WORD.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(DISABLED_WORD))
        {
            return Cow::Borrowed(value);
        }

        let Some(matched) = DISABLED_DETECT_RE.find(value) else {
            return Cow::Borrowed(value);
        };
        value = value[matched.end()..].trim();
    }
}

/// Check if a folder is disabled based on the canonical DISABLED prefix.
pub fn is_disabled_folder(name: &str) -> bool {
    DISABLED_DETECT_RE.is_match(name)
}

/// Common noise words to strip from names before matching.
const NOISE_SKIPWORDS: &[&str] = &[
    "mod", "mods", "skin", "fix", "update", "ver", "version", "v", "by", "disabled",
];

/// Normalize text for substring matching.
///
/// Pipeline:
/// 1. Strip noise/`DISABLED ` prefixes
/// 2. CJK→Latin transliteration (日本語/中文/한국어 → Latin via deunicode)
/// 3. Strip non-alphanumeric characters (keep spaces)
/// 4. Lowercase
/// 5. Strip all digit characters
/// 6. Remove `NOISE_SKIPWORDS`
/// 7. Collapse and trim whitespace
///
/// Returns a continuous cleaned string (NOT tokenized).
pub fn normalize_for_matching_default(text: &str) -> String {
    let stripped = strip_noise_prefixes(text);
    let latin = deunicode(&stripped);
    let clean = RE_NON_ALNUM.replace_all(&latin, " ");
    let no_digits: String = clean
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect();

    no_digits
        .split_whitespace()
        .filter(|word| !NOISE_SKIPWORDS.contains(word))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "tests/normalizer_tests.rs"]
mod tests;
