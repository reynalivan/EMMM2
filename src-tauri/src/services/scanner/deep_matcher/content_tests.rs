use super::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_extract_plain_hash() {
    let text = "hash = d94c8962";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes, vec!["d94c8962"]);
}

#[test]
fn test_extract_hash_with_0x_prefix() {
    let text = "hash = 0xd94c8962";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes, vec!["d94c8962"]);
}

#[test]
fn test_extract_hash_16hex_tail() {
    let text = "hash = 00000000d94c8962";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes, vec!["d94c8962"]);
}

#[test]
fn test_extract_hash_no_space() {
    let text = "hash=abcd1234";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes, vec!["abcd1234"]);
}

#[test]
fn test_extract_hash_uppercase() {
    let text = "HASH = ABC123DE";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes, vec!["abc123de"]);
}

#[test]
fn test_extract_multiple_hashes() {
    let text = "hash = 12345678\nhash = abcdef00";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes.len(), 2);
    assert!(hashes.contains(&"12345678".to_string()));
    assert!(hashes.contains(&"abcdef00".to_string()));
}

#[test]
fn test_extract_ignores_invalid_hash() {
    let text = "hash = invalid\nhash = 12345678";
    let hashes = extract_hashes_from_ini_text(text);
    assert_eq!(hashes, vec!["12345678"]);
}

#[test]
fn test_extract_ignores_non_hash_keys() {
    let text = "[TextureOverride]\nhashing = 12345678\nvalue = abc";
    let hashes = extract_hashes_from_ini_text(text);
    assert!(hashes.is_empty());
}

#[test]
fn test_extract_empty_text() {
    let hashes = extract_hashes_from_ini_text("");
    assert!(hashes.is_empty());
}

#[test]
fn test_normalize_hash_strips_0x() {
    assert_eq!(normalize_hash("0xd94c8962"), Some("d94c8962".to_string()));
    assert_eq!(normalize_hash("0Xabcd1234"), Some("abcd1234".to_string()));
}

#[test]
fn test_normalize_hash_takes_last_8_of_16() {
    assert_eq!(
        normalize_hash("00000000d94c8962"),
        Some("d94c8962".to_string())
    );
}

#[test]
fn test_normalize_hash_rejects_invalid_length() {
    assert_eq!(normalize_hash("abc"), None);
    assert_eq!(normalize_hash("abcdefghijklmnop123"), None);
}

#[test]
fn test_normalize_hash_rejects_non_hex() {
    assert_eq!(normalize_hash("gggggggg"), None);
    assert_eq!(normalize_hash("12345XYZ"), None);
}

#[test]
fn test_normalize_hash_lowercase() {
    assert_eq!(normalize_hash("ABCD1234"), Some("abcd1234".to_string()));
}

#[test]
fn test_decode_utf8_ini() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("test.ini");
    fs::write(&path, "hash = d94c8962\n").expect("write utf8");

    let content = decode_ini_content(&path).expect("decode utf8");
    assert!(content.contains("hash = d94c8962"));
}

#[test]
fn test_decode_empty_ini() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("empty.ini");
    fs::write(&path, "").expect("write empty");

    let content = decode_ini_content(&path).expect("decode empty");
    assert_eq!(content, "");
}

#[test]
fn test_decode_utf16le_with_bom() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("utf16.ini");

    let text = "hash=test";
    let mut utf16_bytes = vec![0xFF, 0xFE];
    for c in text.chars() {
        let word = c as u16;
        utf16_bytes.extend_from_slice(&word.to_le_bytes());
    }

    fs::write(&path, utf16_bytes).expect("write utf16");

    let content = decode_ini_content(&path).expect("decode utf16");
    assert!(content.contains("hash=test"));
}

#[test]
fn test_decode_fallback_lossy() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("invalid.ini");

    fs::write(&path, vec![0xFF, 0xFE, 0xFD, 0x80]).expect("write invalid utf8");

    let content = decode_ini_content(&path).expect("decode fallback");
    assert!(!content.is_empty());
}

#[test]
fn test_decode_nonexistent_file() {
    let result = decode_ini_content(Path::new("/nonexistent/path/file.ini"));
    assert!(result.is_err());
}

#[test]
fn test_extract_from_decoded_utf8() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("config.ini");
    fs::write(
        &path,
        "[TextureOverride]\nhash = 12345678\nhash = abcdef00\n",
    )
    .expect("write config");

    let content = decode_ini_content(&path).expect("decode config");
    let hashes = extract_hashes_from_ini_text(&content);

    assert_eq!(hashes.len(), 2);
    assert!(hashes.contains(&"12345678".to_string()));
    assert!(hashes.contains(&"abcdef00".to_string()));
}

#[test]
fn test_structural_tokenization_extracts_section_key_and_path_buckets() {
    // Covers: TC-2.2-INI-Task7-01
    let text = r#"
        [TextureOverrideDilucVB]
        filename = Characters/Diluc/body_diffuse.dds
        run = 1
        key = 1234
    "#;

    let config = IniTokenizationConfig {
        short_token_whitelist: vec!["vb".to_string()],
        ..Default::default()
    };
    let buckets = extract_structural_ini_tokens(text, &config);

    assert_eq!(
        buckets.section_tokens,
        vec!["diluc".to_string(), "vb".to_string()]
    );
    assert_eq!(buckets.key_tokens, vec!["filename".to_string()]);
    assert!(buckets.path_tokens.contains(&"characters".to_string()));
    assert!(buckets.path_tokens.contains(&"diluc".to_string()));
    assert!(buckets.path_tokens.contains(&"body".to_string()));
    assert!(buckets.path_tokens.contains(&"diffuse".to_string()));
    assert!(!buckets.path_tokens.contains(&"dds".to_string()));
    assert!(!buckets.key_tokens.contains(&"run".to_string()));
}

#[test]
fn test_structural_tokenization_applies_blacklist_and_schema_whitelist() {
    // Covers: TC-2.2-INI-Task7-02
    let text = r#"
        [ShaderOverrideAether]
        character = Aether/avatar.buf
        texture = Travelers/Lumine/tex_diffuse.dds
        run = 1
        metadata = 1234
    "#;

    let config = IniTokenizationConfig {
        ini_key_blacklist: vec!["character".to_string()],
        ini_key_whitelist: vec!["metadata".to_string()],
        short_token_whitelist: vec!["buf".to_string()],
        ..Default::default()
    };
    let buckets = extract_structural_ini_tokens(text, &config);

    assert_eq!(buckets.section_tokens, vec!["aether".to_string()]);
    assert_eq!(buckets.key_tokens, vec!["metadata".to_string()]);
    assert!(buckets.path_tokens.is_empty());
    assert!(!buckets.key_tokens.contains(&"character".to_string()));
    assert!(!buckets.key_tokens.contains(&"texture".to_string()));
}
