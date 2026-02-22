//! Text normalization for mod folder/file names.
//! Handles transliteration, tokenization, and sanitization per TRD §3.2.

use deunicode::deunicode;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Compiled regex for stripping non-alphanumeric characters.
static RE_NON_ALNUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^a-zA-Z0-9\s]").expect("Invalid regex"));

/// Compiled regex for sanitizing filenames (forbidden chars).
static RE_FORBIDDEN_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[\\/:*?"<>|]"#).expect("Invalid regex"));

/// Common noise prefixes to strip from folder names before matching.
const NOISE_PREFIXES: &[&str] = &["[mod]", "[skin]", "[fix]", "[update]", "disabled "];

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
    let mut result = name.to_string();
    let lower = result.to_lowercase();

    for prefix in NOISE_PREFIXES {
        if lower.starts_with(prefix) {
            result = result[prefix.len()..].to_string();
            break;
        }
    }

    result.trim().to_string()
}

/// Sanitize a filename by replacing forbidden characters with `_`.
///
/// Forbidden: `\ / : * ? " < > |`
///
/// # Covers: DI-2.01
pub fn sanitize_filename(name: &str) -> String {
    RE_FORBIDDEN_CHARS.replace_all(name, "_").to_string()
}

/// Normalize a folder name for UI display.
///
/// Strips `DISABLED ` prefix and trims whitespace.
pub fn normalize_display_name(name: &str) -> String {
    let stripped = if let Some(stripped) = name.strip_prefix("DISABLED ") {
        stripped
    } else {
        name
    };
    stripped.trim().to_string()
}

/// Check if a folder is disabled based on the `DISABLED ` prefix convention.
pub fn is_disabled_folder(name: &str) -> bool {
    name.starts_with("DISABLED ")
}

/// Common noise words to strip from names before matching.
const NOISE_SKIPWORDS: &[&str] = &[
    "mod", "mods", "skin", "fix", "update", "ver", "version", "v", "by", "disabled",
];

/// Normalize text for substring matching.
///
/// Pipeline:
/// 1. Strip `DISABLED ` prefix
/// 2. CJK→Latin transliteration (日本語/中文/한국어 → Latin via deunicode)
/// 3. Strip non-alphanumeric characters (keep spaces)
/// 4. Lowercase
/// 5. Strip all digit characters (if `skip_numbers` is true)
/// 6. Remove skipwords
/// 7. Collapse and trim whitespace
///
/// Returns a continuous cleaned string (NOT tokenized).
pub fn normalize_for_matching(text: &str, skip_numbers: bool, skipwords: &[&str]) -> String {
    // Step 1: Strip DISABLED prefix
    let stripped = strip_noise_prefixes(text);

    // Step 2: CJK → Latin transliteration
    let latin = deunicode(&stripped);

    // Step 3: Strip non-alphanumeric (keep spaces)
    let clean = RE_NON_ALNUM.replace_all(&latin, " ");

    // Step 4: Lowercase
    let lower = clean.to_lowercase();

    // Step 5: Strip digits
    let no_digits = if skip_numbers {
        lower
            .chars()
            .filter(|c| !c.is_ascii_digit())
            .collect::<String>()
    } else {
        lower
    };

    // Step 6: Remove skipwords
    let words: Vec<&str> = no_digits
        .split_whitespace()
        .filter(|w| !w.is_empty() && !skipwords.contains(w))
        .collect();

    // Step 7: Collapse whitespace
    words.join(" ")
}

/// Normalize text using the default skipwords list.
pub fn normalize_for_matching_default(text: &str) -> String {
    normalize_for_matching(text, true, NOISE_SKIPWORDS)
}

#[cfg(test)]
#[path = "tests/normalizer_tests.rs"]
mod tests;
