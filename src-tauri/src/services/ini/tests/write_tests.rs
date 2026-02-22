use super::*;
use crate::services::ini::document::read_ini_document;
use std::fs;
use tempfile::TempDir;

// Covers: DI-6.01, TC-6.3-02
#[test]
fn test_save_ini_creates_bak_and_updates_only_target_line() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("config.ini");
    let original = "[Constants]\n$swapvar = 0\n$keep = 9\n";
    fs::write(&ini_path, original).unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    save_ini_with_updates(&document, &[(1, "$swapvar = 1".to_string())]).unwrap();

    let bak_path = backup_path_for(&ini_path).unwrap();
    assert!(bak_path.exists(), "Backup file should exist");

    let backup_content = fs::read_to_string(&bak_path).unwrap();
    assert_eq!(
        backup_content, original,
        "Backup must preserve original bytes"
    );

    let updated_content = fs::read_to_string(&ini_path).unwrap();
    assert!(updated_content.contains("$swapvar = 1"));
    assert!(updated_content.contains("$keep = 9"));
}

// Covers: NC-6.3-01
#[test]
fn test_save_ini_rejects_raw_fallback_document() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("encoded.ini");
    fs::write(&ini_path, [0x82_u8, 0xA0_u8, 0x82_u8]).unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    assert_eq!(document.mode, IniReadMode::RawFallback);

    let result = save_ini_with_updates(&document, &[(0, "key = v".to_string())]);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("raw fallback"),
        "Should block writes when parser fell back to raw mode"
    );
}

// Covers: DI-6.02, TC-6.3-03
#[test]
fn test_save_ini_preserves_bom_when_original_had_bom() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("bom.ini");

    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"[Constants]\n$var = 0\n");
    fs::write(&ini_path, bytes).unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    save_ini_with_updates(&document, &[(1, "$var = 1".to_string())]).unwrap();

    let written = fs::read(&ini_path).unwrap();
    assert!(written.starts_with(&[0xEF, 0xBB, 0xBF]));
}

// Covers: EC-6.05
#[test]
fn test_save_ini_preserves_crlf_style() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("newline.ini");
    fs::write(&ini_path, "[Constants]\r\n$var = 0\r\n").unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    assert_eq!(document.newline_style, NewlineStyle::CrLf);

    save_ini_with_updates(&document, &[(1, "$var = 1".to_string())]).unwrap();

    let written = fs::read_to_string(&ini_path).unwrap();
    assert!(
        written.contains("\r\n"),
        "Expected CRLF newline style to be preserved"
    );
}
