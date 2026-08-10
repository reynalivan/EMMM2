use super::*;
use crate::services::ini::document::{read_ini_document, IniEncoding, NewlineStyle};
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
    save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$swapvar = 1".to_string())],
    )
    .unwrap();

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

    let result = save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(0, "key = v".to_string())],
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("raw fallback"),
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
    save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$var = 1".to_string())],
    )
    .unwrap();

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

    save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$var = 1".to_string())],
    )
    .unwrap();

    let written = fs::read_to_string(&ini_path).unwrap();
    assert!(
        written.contains("\r\n"),
        "Expected CRLF newline style to be preserved"
    );
}

#[test]
fn save_preserves_mixed_terminators_and_final_newline_state() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("mixed.ini");
    fs::write(&ini_path, b"[Constants]\r\n$one = 1\n$two = 2\r$three = 3").unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(2, "$two = 9".to_string())],
    )
    .unwrap();

    assert_eq!(
        fs::read(&ini_path).unwrap(),
        b"[Constants]\r\n$one = 1\n$two = 9\r$three = 3"
    );
}

#[test]
fn save_preserves_shift_jis_encoding() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("shift-jis.ini");
    let source = "[Constants]\r\nglobal $name = 春\r\n";
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(source);
    assert!(!had_errors);
    fs::write(&ini_path, encoded.as_ref()).unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    assert_eq!(document.encoding, IniEncoding::ShiftJis);
    save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "global $name = 夏".to_string())],
    )
    .unwrap();

    let bytes = fs::read(&ini_path).unwrap();
    assert!(String::from_utf8(bytes.clone()).is_err());
    let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
    assert!(!had_errors);
    assert_eq!(decoded, "[Constants]\r\nglobal $name = 夏\r\n");
}

#[test]
fn save_rejects_stale_source_without_creating_backup() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("stale.ini");
    fs::write(&ini_path, "[Constants]\n$value = 0\n").unwrap();
    let document = read_ini_document(&ini_path).unwrap();

    fs::write(&ini_path, "[Constants]\n$value = external\n").unwrap();
    let error = save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$value = editor".to_string())],
    )
    .unwrap_err();

    assert!(error.to_string().contains("changed on disk"));
    assert_eq!(
        fs::read_to_string(&ini_path).unwrap(),
        "[Constants]\n$value = external\n"
    );
    assert!(!backup_path_for(&ini_path).unwrap().exists());
}

#[test]
fn save_rejects_unrepresentable_shift_jis_edit() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("shift-jis.ini");
    let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode("[Constants]\n$name = 春\n");
    fs::write(&ini_path, encoded.as_ref()).unwrap();
    let document = read_ini_document(&ini_path).unwrap();

    let error = save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$name = 😀".to_string())],
    )
    .unwrap_err();
    assert!(error.to_string().contains("Shift-JIS"));
    assert!(!backup_path_for(&ini_path).unwrap().exists());
}

#[test]
fn failed_commit_restores_recovery_file() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("config.ini");
    let recovery = tmp.path().join("config.ini.recover");
    let temp = tmp.path().join("config.ini.tmp");
    fs::write(&recovery, "original").unwrap();
    fs::write(&temp, "replacement").unwrap();

    let result = restore_after_failed_commit(
        &target,
        &recovery,
        &temp,
        std::io::Error::other("injected commit failure"),
    );

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "original");
    assert!(!recovery.exists());
    assert!(!temp.exists());
}
