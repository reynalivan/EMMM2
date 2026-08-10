//! Unit tests for the hash harvester.

use std::fs;
use std::io::Write;
use tempfile::TempDir;

use crate::services::keyviewer::harvester::{harvest_hashes_from_ini, harvest_hashes_from_mod};

/// Helper to create a temp INI file with given content.
fn write_ini(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = fs::File::create(&path).expect("create temp INI");
    f.write_all(content.as_bytes()).expect("write temp INI");
    path
}

#[test]
fn extracts_hash_from_texture_override() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideAlbedoBody]
hash = df65bb00
handling = skip
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
    assert_eq!(result[0].section_name, "TextureOverrideAlbedoBody");
}

#[test]
fn ignores_shader_override_hashes_even_when_they_look_32_bit() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[ShaderOverrideAlbedoVS]
hash = AABBCCDD
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert!(result.is_empty());
}

#[test]
fn skips_non_override_sections() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[Constants]
hash = 12345678

[ResourceAlbedo]
hash = 87654321

[TextureOverrideAlbedo]
hash = df65bb00
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
}

#[test]
fn skips_denylisted_sections() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideNotificationBar]
hash = 11111111

[TextureOverrideUIElement]
hash = 22222222

[TextureOverrideCursorHand]
hash = 33333333

[ShaderOverrideUISomething]
hash = 44444444

[ShaderOverrideShadowMap]
hash = 55555555

[TextureOverrideAlbedo]
hash = df65bb00
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
}

#[test]
fn extracts_multiple_hashes_in_one_file() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideAlbedoHead]
hash = aabbcc11

[TextureOverrideAlbedoBody]
hash = aabbcc22

[TextureOverrideAlbedoDress]
hash = aabbcc33
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 3);
    let hashes: Vec<&str> = result.iter().map(|h| h.hash.as_str()).collect();
    assert!(hashes.contains(&"aabbcc11"));
    assert!(hashes.contains(&"aabbcc22"));
    assert!(hashes.contains(&"aabbcc33"));
}

#[test]
fn handles_inline_comments() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideFoo]
hash = df65bb00 ; position buffer
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
}

#[test]
fn handles_bom_utf8() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bom.ini");
    let mut f = fs::File::create(&path).unwrap();
    // Write UTF-8 BOM + content
    f.write_all(&[0xEF, 0xBB, 0xBF]).unwrap();
    f.write_all(b"[TextureOverrideFoo]\nhash = abcd1234\n")
        .unwrap();

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "abcd1234");
}

#[test]
fn ignores_invalid_hash_lengths() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideFoo]
hash = abc
hash = abcdef1234
hash = 12345678
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "12345678");
}

#[test]
fn harvest_from_mod_aggregates_across_files() {
    let dir = TempDir::new().unwrap();
    write_ini(
        &dir,
        "merged.ini",
        r#"
[TextureOverrideBody]
hash = 11112222
"#,
    );
    write_ini(
        &dir,
        "extra.ini",
        r#"
[TextureOverrideHead]
hash = 33334444

[TextureOverrideExtra]
hash = 11112222
"#,
    );

    let result = harvest_hashes_from_mod(dir.path()).unwrap();
    // "11112222" appears twice (in two files) → grouped
    assert_eq!(result.get("11112222").map(|v| v.len()), Some(2));
    // "33334444" appears once
    assert_eq!(result.get("33334444").map(|v| v.len()), Some(1));
}

#[test]
fn harvest_from_empty_mod_returns_empty() {
    let dir = TempDir::new().unwrap();
    let result = harvest_hashes_from_mod(dir.path()).unwrap();
    assert!(result.is_empty());
}

#[test]
fn handles_case_insensitive_section_names() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[textureoverridealbedobody]
hash = df65bb00

[TEXTUREOVERRIDEALBEDOVS]
hash = aabbccdd
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 2);
}
