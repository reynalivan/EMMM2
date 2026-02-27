use super::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const ARCHIVE_BACKUP_DIR: &str = ".archive_backup";

/// Helper: create a minimal valid ZIP.
fn create_test_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (entry_name, content) in files {
        writer.start_file(entry_name.to_string(), options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap();
    zip_path
}

#[test]
fn test_format_detection() {
    assert_eq!(
        ArchiveFormat::from_path(Path::new("mod.zip")),
        Some(ArchiveFormat::Zip)
    );
    assert_eq!(
        ArchiveFormat::from_path(Path::new("mod.7z")),
        Some(ArchiveFormat::SevenZ)
    );
    assert_eq!(
        ArchiveFormat::from_path(Path::new("mod.rar")),
        Some(ArchiveFormat::Rar)
    );
    assert_eq!(ArchiveFormat::from_path(Path::new("mod.txt")), None);
}

// Covers: US-2.1 Pre-Extraction Analysis (ZIP)
#[test]
fn test_analyze_zip_archive() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "test.zip",
        &[
            ("config.ini", b"[Section]\nkey=val"),
            ("texture.dds", b"binary data"),
        ],
    );

    let analysis = analyze_archive(&zip_path).unwrap();
    assert_eq!(analysis.format, ArchiveFormat::Zip);
    assert_eq!(analysis.file_count, 2);
    assert!(analysis.has_ini);
}

#[test]
fn test_analyze_zip_single_root() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "wrapped.zip",
        &[
            ("ModFolder/config.ini", b"data"),
            ("ModFolder/tex.dds", b"data"),
        ],
    );

    let analysis = analyze_archive(&zip_path).unwrap();
    assert_eq!(analysis.single_root_folder, Some("ModFolder".to_string()));
}

// Covers: TC-2.1-01 — Basic ZIP extraction
#[test]
fn test_extract_zip_basic() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "mod_pack.zip",
        &[
            ("config.ini", b"[Section]\nkey=val"),
            ("texture.dds", b"binary data"),
        ],
    );

    let result = extract_archive(&zip_path, dir.path(), None, false).unwrap();
    assert!(result.success);
    assert_eq!(result.files_extracted, 2);
    assert!(dir.path().join("mod_pack").join("config.ini").exists());

    // Archive moved to backup
    assert!(!zip_path.exists());
    assert!(dir
        .path()
        .join(ARCHIVE_BACKUP_DIR)
        .join("mod_pack.zip")
        .exists());
}

// Covers: TC-2.1-02 — Smart flattening
#[test]
fn test_extract_zip_smart_flatten() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "nested_mod.zip",
        &[
            ("WrapperFolder/config.ini", b"data"),
            ("WrapperFolder/sub/texture.dds", b"data"),
        ],
    );

    let result = extract_archive(&zip_path, dir.path(), None, false).unwrap();
    assert!(result.success);

    let dest = dir.path().join("nested_mod");
    assert!(dest.join("config.ini").exists());
    assert!(dest.join("sub").join("texture.dds").exists());
    assert!(!dest.join("WrapperFolder").exists());
}

// Covers: EC-2.07 — Duplicate destination
#[test]
fn test_extract_duplicate_dest() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(dir.path(), "existing_mod.zip", &[("file.txt", b"data")]);

    fs::create_dir(dir.path().join("existing_mod")).unwrap();

    // Without overwrite (auto-renames to "existing_mod (1)")
    let result = extract_archive(&zip_path, dir.path(), None, false).unwrap();
    assert!(result.success);
    assert!(result.dest_path.ends_with("existing_mod (1)"));
    assert!(dir
        .path()
        .join("existing_mod (1)")
        .join("file.txt")
        .exists());

    // Re-create the test zip since extract_archive moves it to backup
    let zip_path2 = create_test_zip(dir.path(), "existing_mod2.zip", &[("file.txt", b"data")]);

    // With overwrite
    assert!(zip_path2.exists());
    let result2 = extract_archive(&zip_path2, dir.path(), None, true).unwrap();
    assert!(result2.success);
    // Since overwrite is true, it extracts to "existing_mod2" directly
    assert!(dir.path().join("existing_mod2").join("file.txt").exists());
}

// Covers: NC-2.1-01 — Corrupt archive
#[test]
fn test_extract_corrupt_archive() {
    let dir = TempDir::new().unwrap();
    let zip_path = dir.path().join("corrupt.zip");
    fs::write(&zip_path, b"not a real zip file").unwrap();

    let result = extract_archive(&zip_path, dir.path(), None, false);
    assert!(result.is_err());
}

#[test]
fn test_unsupported_format() {
    let result = analyze_archive(Path::new("file.tar.gz"));
    assert!(result.is_err());
}

// ─── 7z Tests ──────────────────────────────────────────────────
// Note: Kept lighter tests here, removed heavy 7z generation helpers from analysis
// to keep mod.rs smaller. Integration tests cover heavy scenarios.

/// Helper: create a password-protected ZIP.
fn create_encrypted_zip(
    dir: &Path,
    name: &str,
    password: &str,
    files: &[(&str, &[u8])],
) -> PathBuf {
    use zip::unstable::write::FileOptionsExt;
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .with_deprecated_encryption(password.as_bytes());

    for (entry_name, content) in files {
        writer.start_file(entry_name.to_string(), options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap();
    zip_path
}

// Covers: TC-2.1-04 — ZIP password extraction
#[test]
fn test_extract_zip_with_password() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_encrypted_zip(
        dir.path(),
        "secret.zip",
        "mypassword",
        &[("data.txt", b"secret content")],
    );

    let result = extract_archive(&zip_path, dir.path(), Some("mypassword"), false).unwrap();
    assert!(result.success);
    assert_eq!(result.files_extracted, 1);

    let extracted = dir.path().join("secret").join("data.txt");
    assert!(extracted.exists());
    assert_eq!(fs::read_to_string(&extracted).unwrap(), "secret content");
}
