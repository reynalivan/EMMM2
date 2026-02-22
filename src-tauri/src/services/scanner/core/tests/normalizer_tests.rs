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
