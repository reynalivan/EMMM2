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

#[cfg(test)]
mod tests {
    use super::*;

    // Covers: Epic 2 §B.1 — Basic tokenization
    #[test]
    fn test_preprocess_text_basic() {
        let tokens = preprocess_text("Raiden Shogun");
        assert!(tokens.contains("raiden"));
        assert!(tokens.contains("shogun"));
        assert_eq!(tokens.len(), 2);
    }

    // Covers: Epic 2 §B.1 — Symbol stripping
    #[test]
    fn test_preprocess_text_with_symbols() {
        let tokens = preprocess_text("[Mod] Raiden_Shogun-v2.0");
        assert!(tokens.contains("mod"));
        assert!(tokens.contains("raiden"));
        assert!(tokens.contains("shogun"));
        assert!(tokens.contains("v2"));
        assert!(tokens.contains("0"));
    }

    // Covers: EC-2.03 — CJK transliteration
    #[test]
    fn test_preprocess_text_cjk() {
        let tokens = preprocess_text("神里綾華");
        // deunicode converts CJK to romanized approximation
        assert!(!tokens.is_empty());
    }

    // Covers: Epic 2 §B.1 — Empty string
    #[test]
    fn test_preprocess_text_empty() {
        let tokens = preprocess_text("");
        assert!(tokens.is_empty());
    }

    // Covers: DI-2.01 — Filename sanitization
    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename(r#"mod:name*test"file"#),
            "mod_name_test_file"
        );
        assert_eq!(sanitize_filename("normal_name"), "normal_name");
        assert_eq!(sanitize_filename("path\\to/file"), "path_to_file");
    }

    #[test]
    fn test_normalize_display_name() {
        assert_eq!(normalize_display_name("DISABLED raiden_mod"), "raiden_mod");
        assert_eq!(normalize_display_name("raiden_mod"), "raiden_mod");
    }

    #[test]
    fn test_is_disabled_folder() {
        assert!(is_disabled_folder("DISABLED some_mod"));
        assert!(!is_disabled_folder("some_mod"));
        assert!(!is_disabled_folder("disabled some_mod")); // Case-sensitive
    }

    #[test]
    fn test_strip_noise_prefixes() {
        assert_eq!(strip_noise_prefixes("[Mod] Raiden Shogun"), "Raiden Shogun");
        assert_eq!(strip_noise_prefixes("DISABLED Ayaka Mod"), "Ayaka Mod");
        assert_eq!(strip_noise_prefixes("plain_name"), "plain_name");
    }
}
