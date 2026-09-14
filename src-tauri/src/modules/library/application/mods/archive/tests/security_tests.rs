use super::*;
use crate::shared::errors::AppError;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

fn create_zip(
    dir: &Path,
    name: &str,
    method: zip::CompressionMethod,
    files: &[(&str, &[u8])],
) -> PathBuf {
    let path = dir.join(name);
    let file = fs::File::create(&path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    for (entry_name, contents) in files {
        writer.start_file(*entry_name, options).unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap();
    path
}

fn assert_security(error: AppError) {
    assert!(matches!(error, AppError::Security(_)), "{error}");
}

fn assert_limit(error: AppError, limit_name: &str) {
    assert!(matches!(error, AppError::Validation(_)), "{error}");
    assert!(error.to_string().contains(limit_name), "{error}");
}

#[test]
fn archive_entry_paths_reject_escape_and_windows_aliases() {
    let root = Path::new(r"C:\staging");
    for entry in [
        "/absolute.ini",
        r"C:\prefix.ini",
        r"\\server\share\file.ini",
        "../parent.ini",
        "safe/../parent.ini",
        "nul\0byte.ini",
        "Mod/file.ini:stream",
        "CON",
        "con.txt",
        "AUX/config.ini",
        "PRN.ini",
        "COM1/file.ini",
        "LPT9.txt",
        "trailing-dot./file.ini",
        "trailing-space /file.ini",
    ] {
        let error =
            super::security::validate_entry_path(root, entry, super::security::EntryKind::File)
                .unwrap_err();
        assert_security(error);
    }

    assert_eq!(
        super::security::validate_entry_path(
            root,
            "SafeMod/config.ini",
            super::security::EntryKind::File,
        )
        .unwrap(),
        root.join("SafeMod").join("config.ini")
    );
    assert_eq!(
        super::security::validate_entry_path(
            root,
            "SafeMod/",
            super::security::EntryKind::Directory,
        )
        .unwrap(),
        root.join("SafeMod")
    );
    let error =
        super::security::validate_entry_path(root, "SafeMod/", super::security::EntryKind::File)
            .unwrap_err();
    assert_security(error);
    let error = super::security::validate_entry_path(
        root,
        "SafeMod//",
        super::security::EntryKind::Directory,
    )
    .unwrap_err();
    assert_security(error);
}

#[test]
fn staging_accepts_explicit_zip_directory_entries() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("directory.zip");
    let file = fs::File::create(&archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    writer.add_directory("Mod/", options).unwrap();
    writer.start_file("Mod/config.ini", options).unwrap();
    writer.write_all(b"[TextureOverride]\n").unwrap();
    writer.finish().unwrap();

    let staging = dir.path().join("staging");
    let result = extract_archive_to_staging(&archive, &staging).unwrap();

    assert_eq!(result.mod_roots, vec![staging.join("Mod")]);
}

#[test]
fn staging_rejects_executable_archive_entries() {
    let dir = TempDir::new().unwrap();
    let archive = create_zip(
        dir.path(),
        "unsafe.zip",
        zip::CompressionMethod::Stored,
        &[
            ("Mod/config.ini", b"[TextureOverride]\n"),
            ("Mod/loader.dll", b"binary"),
        ],
    );
    let staging = dir.path().join("staging");

    let error = extract_archive_to_staging(&archive, &staging).unwrap_err();

    assert_security(error);
    assert!(!staging.exists());
}

#[test]
fn archive_entry_types_reject_links_and_special_files() {
    assert!(super::security::validate_entry_type(0o100644, 1).is_ok());
    assert!(super::security::validate_entry_type(0o040755, 1).is_ok());

    for (mode, links) in [
        (0o120777, 1),
        (0o100644, 2),
        (0o060600, 1),
        (0o020600, 1),
        (0o010600, 1),
        (0o140600, 1),
    ] {
        let error = super::security::validate_entry_type(mode, links).unwrap_err();
        assert_security(error);
    }
}

#[test]
fn archive_budget_checks_actual_chunks_and_cancellation() {
    let limits = ArchiveLimits {
        max_entries: 2,
        max_single_file_bytes: 8,
        max_total_bytes: 12,
        max_compression_ratio: 10,
    };
    let mut budget = super::security::ExtractionBudget::new(limits, 100);
    budget.start_entry(1).unwrap();
    budget.consume_chunk(8, &None).unwrap();

    let error = budget.consume_chunk(1, &None).unwrap_err();
    assert_limit(error, "single-file");

    let token = Arc::new(AtomicBool::new(true));
    let error = budget.consume_chunk(1, &Some(token)).unwrap_err();
    assert!(matches!(error, AppError::Cancelled));
}

#[test]
fn staging_rejects_symlink_entries_and_removes_partial_output() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("symlink.zip");
    let file = fs::File::create(&archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("Mod/config.ini", options).unwrap();
    writer.write_all(b"[TextureOverride]\n").unwrap();
    writer
        .add_symlink("Mod/escape", "../../outside", options)
        .unwrap();
    writer.finish().unwrap();
    let staging = dir.path().join("staging");

    let error = extract_archive_to_staging(&archive, &staging).unwrap_err();

    assert_security(error);
    assert!(!staging.exists());
    assert!(archive.exists());
}

#[test]
fn staging_enforces_configurable_entry_limit() {
    let dir = TempDir::new().unwrap();
    let archive = create_zip(
        dir.path(),
        "entries.zip",
        zip::CompressionMethod::Stored,
        &[
            ("Mod/config.ini", b"[TextureOverride]\n"),
            ("Mod/a.bin", b"a"),
        ],
    );
    let staging = dir.path().join("staging");
    let limits = ArchiveLimits {
        max_entries: 1,
        ..ArchiveLimits::default()
    };

    let error = extract_archive_to_staging_with_limits(
        &archive,
        &staging,
        StagingExtractOptions::default(),
        limits,
    )
    .unwrap_err();

    assert_limit(error, "entry-count");
    assert!(!staging.exists());
}

#[test]
fn staging_enforces_single_total_and_ratio_limits() {
    let dir = TempDir::new().unwrap();
    let compressible = vec![0_u8; 16 * 1024];
    let archive = create_zip(
        dir.path(),
        "limits.zip",
        zip::CompressionMethod::Deflated,
        &[
            ("Mod/config.ini", b"[TextureOverride]\n"),
            ("Mod/data.bin", &compressible),
        ],
    );

    for (name, limits, expected) in [
        (
            "single",
            ArchiveLimits {
                max_single_file_bytes: 1024,
                ..ArchiveLimits::default()
            },
            "single-file",
        ),
        (
            "total",
            ArchiveLimits {
                max_total_bytes: 1024,
                ..ArchiveLimits::default()
            },
            "total-bytes",
        ),
        (
            "ratio",
            ArchiveLimits {
                max_compression_ratio: 1,
                ..ArchiveLimits::default()
            },
            "compression-ratio",
        ),
    ] {
        let staging = dir.path().join(name);
        let error = extract_archive_to_staging_with_limits(
            &archive,
            &staging,
            StagingExtractOptions::default(),
            limits,
        )
        .unwrap_err();
        assert_limit(error, expected);
        assert!(!staging.exists());
    }
}

#[test]
fn multi_volume_archive_names_are_rejected_explicitly() {
    let dir = TempDir::new().unwrap();
    for name in ["pack.part1.rar", "pack.z01", "pack.r00", "pack.001"] {
        let path = dir.path().join(name);
        fs::write(&path, b"not used").unwrap();
        let error = analyze_archive(&path).unwrap_err();
        assert!(matches!(error, AppError::Validation(_)), "{error}");
        assert!(error.to_string().contains("Multi-volume"), "{error}");
    }
}

#[test]
fn chunk_budget_observes_cancellation_between_chunks() {
    let token = Arc::new(AtomicBool::new(false));
    let mut budget = super::security::ExtractionBudget::new(ArchiveLimits::default(), 1024);
    budget.start_entry(16).unwrap();
    budget.consume_chunk(8, &Some(token.clone())).unwrap();
    token.store(true, Ordering::SeqCst);

    let error = budget.consume_chunk(8, &Some(token)).unwrap_err();

    assert!(matches!(error, AppError::Cancelled));
}

#[test]
fn analysis_enforces_configurable_limits_and_cancellation() {
    let dir = TempDir::new().unwrap();
    let compressible = vec![0_u8; 16 * 1024];
    let archive = create_zip(
        dir.path(),
        "analysis-limits.zip",
        zip::CompressionMethod::Deflated,
        &[
            ("Mod/config.ini", b"[TextureOverride]\n"),
            ("Mod/data.bin", &compressible),
        ],
    );

    for (limits, expected) in [
        (
            ArchiveLimits {
                max_entries: 1,
                ..ArchiveLimits::default()
            },
            "entry-count",
        ),
        (
            ArchiveLimits {
                max_single_file_bytes: 1024,
                ..ArchiveLimits::default()
            },
            "single-file",
        ),
        (
            ArchiveLimits {
                max_total_bytes: 1024,
                ..ArchiveLimits::default()
            },
            "total-bytes",
        ),
        (
            ArchiveLimits {
                max_compression_ratio: 1,
                ..ArchiveLimits::default()
            },
            "compression-ratio",
        ),
    ] {
        let error =
            super::analyze::analyze_archive_with_limits(&archive, limits, None).unwrap_err();
        assert_limit(error, expected);
    }

    let cancel_token = Arc::new(AtomicBool::new(true));
    let error = super::analyze::analyze_archive_with_limits(
        &archive,
        ArchiveLimits::default(),
        Some(cancel_token),
    )
    .unwrap_err();
    assert!(matches!(error, AppError::Cancelled));
}

#[test]
fn nested_extraction_uses_one_entry_and_byte_budget() {
    let dir = TempDir::new().unwrap();
    let payload = vec![7_u8; 1024];
    let inner = create_zip(
        dir.path(),
        "inner.zip",
        zip::CompressionMethod::Stored,
        &[("NestedMod/config.ini", &payload)],
    );
    let inner_bytes = fs::read(inner).unwrap();
    let config = b"[TextureOverride]\n";
    let outer = create_zip(
        dir.path(),
        "outer.zip",
        zip::CompressionMethod::Stored,
        &[("OuterMod/config.ini", config), ("inner.zip", &inner_bytes)],
    );

    let entry_error = extract_archive_to_staging_with_limits(
        &outer,
        &dir.path().join("entry-budget"),
        StagingExtractOptions::default(),
        ArchiveLimits {
            max_entries: 2,
            ..ArchiveLimits::default()
        },
    )
    .unwrap_err();
    assert_limit(entry_error, "entry-count");

    let byte_error = extract_archive_to_staging_with_limits(
        &outer,
        &dir.path().join("byte-budget"),
        StagingExtractOptions::default(),
        ArchiveLimits {
            max_total_bytes: inner_bytes.len() as u64 + config.len() as u64,
            ..ArchiveLimits::default()
        },
    )
    .unwrap_err();
    assert_limit(byte_error, "total-bytes");
}
