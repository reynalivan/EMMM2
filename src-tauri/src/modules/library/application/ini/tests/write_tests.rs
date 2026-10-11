use super::*;
use crate::modules::library::application::ini::document::{
    read_ini_document, IniEncoding, NewlineStyle,
};
use std::fs;
use tempfile::TempDir;

#[cfg(windows)]
#[test]
fn locked_initial_ini_read_preserves_lock_diagnostics_and_bytes() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = TempDir::new().unwrap();
    let path = fixture.path().join("config.ini");
    let original = "[Constants]\n$value = 1\n";
    fs::write(&path, original).unwrap();
    let document = read_ini_document(&path).unwrap();
    let holder = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let error = save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$value = 2".into())],
    )
    .unwrap_err();
    assert!(
        matches!(error, AppError::PathBusy { path: locked } if locked == path.to_string_lossy())
    );
    drop(holder);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!backup_path_for(&path).unwrap().exists());
}

#[cfg(windows)]
#[test]
fn restored_ini_commit_retains_lock_diagnostics() {
    let fixture = TempDir::new().unwrap();
    let target = fixture.path().join("config.ini");
    let recovery = fixture.path().join("config.ini.bak");
    let temp = fixture.path().join("config.ini.tmp");
    fs::write(&recovery, "original").unwrap();
    fs::write(&temp, "replacement").unwrap();
    let source = open_save_file(&recovery, false).unwrap();
    let parent = crate::platform::fs::rename::PinnedDirectory::open(fixture.path()).unwrap();
    let error = restore_after_failed_commit(
        &target,
        &recovery,
        &temp,
        std::io::Error::from_raw_os_error(32),
        &source,
        &parent,
    )
    .unwrap_err();
    assert!(matches!(error, AppError::PathBusy { path } if path == target.to_string_lossy()));
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert!(!recovery.exists());
    assert!(!temp.exists());
}

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

#[test]
fn save_ini_skips_backup_and_replacement_for_a_noop_line_update() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("config.ini");
    let original = "[Constants]\n$swapvar = 0\n";
    fs::write(&ini_path, original).unwrap();

    let document = read_ini_document(&ini_path).unwrap();
    save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$swapvar = 0".to_string())],
    )
    .unwrap();

    assert_eq!(fs::read_to_string(&ini_path).unwrap(), original);
    assert!(
        !backup_path_for(&ini_path).unwrap().exists(),
        "a no-op must not create a backup or replace the source file"
    );
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

    let source = open_save_file(&recovery, false).unwrap();
    let parent = crate::platform::fs::rename::PinnedDirectory::open(tmp.path()).unwrap();
    let result = restore_after_failed_commit(
        &target,
        &recovery,
        &temp,
        std::io::Error::other("injected commit failure"),
        &source,
        &parent,
    );

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "original");
    assert!(!recovery.exists());
    assert!(!temp.exists());
}

#[test]
fn canary_ini_save_never_discards_a_concurrent_external_edit() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let mut observed = false;
    for _ in 0..12 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("mod.ini");
        fs::write(
            &path,
            format!("[KeyTest]\nkey = F1\n;{}\n", "padding".repeat(200_000)),
        )
        .unwrap();
        let document = read_ini_document(&path).unwrap();
        let finished = AtomicBool::new(false);
        let external = b"[KeyTest]\nkey = F9\n;external editor\n";
        std::thread::scope(|scope| {
            let observer = scope.spawn(|| {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !finished.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
                    if fs::read_dir(root.path()).unwrap().flatten().any(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with("mod.ini.tmp.")
                    }) {
                        return Some(fs::write(&path, external).is_ok());
                    }
                    std::thread::yield_now();
                }
                None
            });
            let _saved = save_ini_with_updates(
                &document,
                &document.source_hash,
                &[(1, "key = F2".to_string())],
            );
            finished.store(true, Ordering::Release);
            if let Some(written) = observer.join().unwrap() {
                observed = true;
                if written {
                    let current = fs::read(&path).unwrap();
                    let backup = fs::read(root.path().join("mod.ini.bak")).unwrap_or_default();
                    assert!(
                        current == external || backup == external,
                        "newer external bytes must remain recoverable"
                    );
                }
            }
        });
        if observed {
            break;
        }
    }
    assert!(observed, "observer must reach the active save window");
}

#[test]
fn canary_ini_rollback_preserves_a_recreated_destination() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("config.ini");
    let recovery = root.path().join("config.ini.recover");
    let temp = root.path().join("config.ini.tmp");
    fs::write(&target, "external replacement").unwrap();
    fs::write(&recovery, "original").unwrap();
    fs::write(&temp, "replacement").unwrap();
    let source = open_save_file(&recovery, false).unwrap();
    let parent = crate::platform::fs::rename::PinnedDirectory::open(root.path()).unwrap();
    assert!(restore_after_failed_commit(
        &target,
        &recovery,
        &temp,
        std::io::Error::other("injected"),
        &source,
        &parent
    )
    .is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "external replacement");
    assert_eq!(fs::read_to_string(&recovery).unwrap(), "original");
}

#[test]
fn noop_save_does_not_require_write_access_to_the_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("readonly.ini");
    fs::write(&path, "[Constants]\n$value = 1\n").unwrap();
    let document = read_ini_document(&path).unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = save_ini_with_updates(
        &document,
        &document.source_hash,
        &[(1, "$value = 1".to_string())],
    );
    fs::set_permissions(&path, original_permissions).unwrap();
    assert!(result.is_ok());
    assert!(!backup_path_for(&path).unwrap().exists());
}

#[test]
fn backup_read_failure_restores_the_live_ini() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("config.ini");
    let backup = root.path().join("config.ini.bak");
    let temp = root.path().join("config.ini.tmp");
    fs::write(&backup, "original").unwrap();
    fs::write(&temp, "replacement").unwrap();
    let source = open_save_file(&backup, false).unwrap();
    let replacement = open_save_file(&temp, false).unwrap();
    let parent = crate::platform::fs::rename::PinnedDirectory::open(root.path()).unwrap();
    let result = finish_ini_commit(
        &source,
        replacement,
        &target,
        &backup,
        &temp,
        &parent,
        Err(std::io::Error::other("injected backup read failure")),
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("injected backup read failure"));
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert!(!backup.exists());
    assert!(!temp.exists());
}
