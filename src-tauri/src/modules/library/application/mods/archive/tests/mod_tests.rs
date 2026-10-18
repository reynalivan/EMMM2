use super::*;
use crate::shared::errors::AppError;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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
fn staging_preserves_cp437_asset_names_that_also_form_valid_utf8() {
    let dir = TempDir::new().unwrap();
    let ini = "[TextureOverrideLegacy]\nhash = 12345678\n[ResourceLegacy]\nfilename = ├⌐.dds\n";
    let zip_path = create_test_zip(
        dir.path(),
        "legacy-name.zip",
        &[("AB.dds", b"texture"), ("config.ini", ini.as_bytes())],
    );
    let mut bytes = fs::read(&zip_path).unwrap();
    let placeholder = b"AB.dds";
    let offsets: Vec<_> = bytes
        .windows(placeholder.len())
        .enumerate()
        .filter_map(|(index, value)| (value == placeholder).then_some(index))
        .collect();
    // Rewrite local/central names without changing the ASCII writer's encoding flag.
    assert_eq!(offsets.len(), 2);
    for offset in offsets {
        bytes[offset..offset + 2].copy_from_slice(&[0xc3, 0xa9]);
    }
    fs::write(&zip_path, bytes).unwrap();
    let staging_dir = dir.path().join("staging");

    extract_archive_to_staging(&zip_path, &staging_dir).unwrap();

    assert!(staging_dir.join("├⌐.dds").is_file());
    assert!(!staging_dir.join("é.dds").exists());
    assert_eq!(
        fs::read_to_string(staging_dir.join("config.ini")).unwrap(),
        ini
    );
}

#[test]
fn staging_pre_cancelled_aborts_and_cleans_up() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "cancelled-staging.zip",
        &[("config.ini", b"[TextureOverride]\ndata")],
    );
    let staging_dir = dir.path().join("staging");
    let cancel_token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

    let error = extract_archive_to_staging_with_options(
        &zip_path,
        &staging_dir,
        StagingExtractOptions {
            cancel_token: Some(cancel_token),
            ..Default::default()
        },
    )
    .unwrap_err();

    assert!(matches!(error, AppError::Cancelled));
    assert!(!staging_dir.exists());
    assert!(zip_path.exists());
}

#[test]
fn staging_unpacks_nested_archives_by_default() {
    let dir = TempDir::new().unwrap();
    let inner_zip = create_test_zip(
        dir.path(),
        "nested-inner.zip",
        &[("NestedMod/config.ini", b"[TextureOverride]\nnested")],
    );
    let inner_bytes = fs::read(&inner_zip).unwrap();
    let outer_zip = create_test_zip(
        dir.path(),
        "nested-outer.zip",
        &[("nested-inner.zip", &inner_bytes)],
    );
    let staging_dir = dir.path().join("staging");

    let staged = extract_archive_to_staging(&outer_zip, &staging_dir).unwrap();

    assert_eq!(staged.mod_roots.len(), 1);
    assert!(staging_dir
        .join("nested-inner")
        .join("NestedMod")
        .join("config.ini")
        .exists());
    assert!(outer_zip.exists());
}

#[test]
fn test_format_detection() {
    assert_eq!(
        ArchiveFormat::detect(Path::new("mod.zip")),
        Some(ArchiveFormat::Zip)
    );
    assert_eq!(
        ArchiveFormat::detect(Path::new("mod.7z")),
        Some(ArchiveFormat::SevenZ)
    );
    assert_eq!(
        ArchiveFormat::detect(Path::new("mod.rar")),
        Some(ArchiveFormat::Rar)
    );
    assert_eq!(ArchiveFormat::detect(Path::new("mod.txt")), None);
}

// Covers: US-2.1 Pre-Extraction Analysis (ZIP)
#[test]
fn test_analyze_zip_archive() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "test.zip",
        &[
            ("config.ini", b"[TextureOverride]\nkey=val"),
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
            ("ModFolder/config.ini", b"[TextureOverride]\ndata"),
            ("ModFolder/tex.dds", b"data"),
        ],
    );

    let analysis = analyze_archive(&zip_path).unwrap();
    assert_eq!(analysis.single_root_folder, Some("ModFolder".to_string()));
}

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

#[test]
fn staging_password_reaches_zip_extractor_and_reports_progress() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_encrypted_zip(
        dir.path(),
        "staging-secret.zip",
        "correct-password",
        &[("SecretMod/config.ini", b"[TextureOverride]\nsecret")],
    );
    let staging_dir = dir.path().join("staging-secret");
    let progress_seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let progress_for_channel = progress_seen.clone();
    let on_progress = tauri::ipc::Channel::new(move |_| {
        progress_for_channel.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    });

    let staged = extract_archive_to_staging_with_options(
        &zip_path,
        &staging_dir,
        StagingExtractOptions {
            password: Some("correct-password".to_string()),
            on_progress: Some(on_progress),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(staged.files_extracted, 1);
    assert!(staging_dir.join("SecretMod").join("config.ini").exists());
    assert_eq!(
        fs::read(staging_dir.join("SecretMod/config.ini")).unwrap(),
        b"[TextureOverride]\nsecret"
    );
    assert!(progress_seen.load(std::sync::atomic::Ordering::SeqCst));
    assert!(zip_path.exists());
}

#[test]
fn staging_propagates_the_supplied_password_to_nested_archives() {
    let dir = TempDir::new().unwrap();
    let inner_zip = create_encrypted_zip(
        dir.path(),
        "nested-secret.zip",
        "same-password",
        &[("NestedSecret/config.ini", b"[TextureOverride]\nsecret")],
    );
    let inner_bytes = fs::read(inner_zip).unwrap();
    let outer_zip = create_test_zip(
        dir.path(),
        "outer-secret.zip",
        &[
            ("OuterMod/config.ini", b"[TextureOverride]\nouter"),
            ("nested-secret.zip", &inner_bytes),
        ],
    );
    let staging_dir = dir.path().join("nested-secret-staging");

    let staged = extract_archive_to_staging_with_options(
        &outer_zip,
        &staging_dir,
        StagingExtractOptions {
            password: Some("same-password".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(staged.files_extracted, 3);
    assert!(staging_dir
        .join("nested-secret")
        .join("NestedSecret")
        .join("config.ini")
        .exists());
}

#[test]
fn test_unsupported_format() {
    let result = analyze_archive(Path::new("file.tar.gz"));
    assert!(result.is_err());
}

#[test]
fn staging_extracts_flat_mod_without_mutating_the_source_archive() {
    let dir = TempDir::new().unwrap();
    let archive = create_test_zip(
        dir.path(),
        "mod_pack.zip",
        &[
            ("config.ini", b"[TextureOverride]\nkey=val"),
            ("texture.dds", b"binary data"),
            ("README.txt", b"author notes"),
        ],
    );
    let source_bytes = fs::read(&archive).unwrap();
    let staging = dir.path().join("staging");
    let staged = extract_archive_to_staging(&archive, &staging).unwrap();
    assert_eq!(staged.mod_roots, vec![staging.clone()]);
    assert_eq!(staged.files_extracted, 3);
    assert_eq!(
        fs::read(staging.join("config.ini")).unwrap(),
        b"[TextureOverride]\nkey=val"
    );
    assert_eq!(
        fs::read(staging.join("README.txt")).unwrap(),
        b"author notes"
    );
    assert_eq!(fs::read(archive).unwrap(), source_bytes);
    assert!(!dir.path().join(".extracted").exists());
}

#[test]
fn staging_discovers_wrapped_roots_and_preserves_their_assets() {
    let dir = TempDir::new().unwrap();
    let archive = create_test_zip(
        dir.path(),
        "nested_mod.zip",
        &[
            ("WrapperFolder/config.ini", b"[TextureOverride]\ndata"),
            ("WrapperFolder/sub/texture.dds", b"data"),
        ],
    );
    let staging = dir.path().join("staging");
    let staged = extract_archive_to_staging(&archive, &staging).unwrap();
    assert_eq!(staged.mod_roots, vec![staging.join("WrapperFolder")]);
    assert_eq!(
        fs::read(staged.mod_roots[0].join("sub/texture.dds")).unwrap(),
        b"data"
    );
    assert!(archive.exists());
}

#[test]
fn staging_rejects_corrupt_archives_without_leaving_output() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("corrupt.zip");
    fs::write(&archive, b"not a real zip file").unwrap();
    let staging = dir.path().join("staging");
    assert!(extract_archive_to_staging(&archive, &staging).is_err());
    assert!(!staging.exists());
    assert!(archive.exists());
}

#[test]
fn staging_wrong_password_fails_and_preserves_the_archive() {
    let dir = TempDir::new().unwrap();
    let archive = create_encrypted_zip(
        dir.path(),
        "locked.zip",
        "correct-password",
        &[("config.ini", b"[TextureOverride]\nsecret")],
    );
    let staging = dir.path().join("staging");
    assert!(extract_archive_to_staging_with_options(
        &archive,
        &staging,
        StagingExtractOptions {
            password: Some("wrong-password".to_string()),
            ..Default::default()
        }
    )
    .is_err());
    assert!(!staging.exists());
    assert!(archive.exists());
}

#[test]
fn staging_nested_archive_depth_limit_cleans_every_extracted_layer() {
    let dir = TempDir::new().unwrap();
    let bottom = create_test_zip(dir.path(), "level4.zip", &[("bottom.txt", b"too deep")]);
    let third = create_test_zip(
        dir.path(),
        "level3.zip",
        &[("level4.zip", &fs::read(bottom).unwrap())],
    );
    let second = create_test_zip(
        dir.path(),
        "level2.zip",
        &[
            ("level3.zip", &fs::read(third).unwrap()),
            ("config.ini", b"[TextureOverride]\nvalid"),
        ],
    );
    let outer = create_test_zip(
        dir.path(),
        "level1.zip",
        &[("level2.zip", &fs::read(second).unwrap())],
    );
    let staging = dir.path().join("staging");
    let error = extract_archive_to_staging(&outer, &staging).unwrap_err();
    assert!(error.to_string().contains("nested_archive_depth_limit"));
    assert!(!staging.exists());
    assert!(outer.exists());
}

#[test]
fn staging_never_overwrites_an_existing_destination() {
    let dir = TempDir::new().unwrap();
    let archive = create_test_zip(
        dir.path(),
        "existing_mod.zip",
        &[("config.ini", b"[TextureOverride]\nnew")],
    );
    let staging = dir.path().join("existing");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("keep.ini"), b"original").unwrap();
    assert!(matches!(
        extract_archive_to_staging(&archive, &staging),
        Err(AppError::Validation(_))
    ));
    assert_eq!(fs::read(staging.join("keep.ini")).unwrap(), b"original");
    assert!(!staging.join("config.ini").exists());
    assert!(archive.exists());
}

#[test]
fn staging_rejects_invalid_mods_and_cleans_partial_output() {
    let dir = TempDir::new().unwrap();
    let archive = create_test_zip(
        dir.path(),
        "invalid.zip",
        &[("readme.txt", b"no runnable INI")],
    );
    let staging = dir.path().join("staging");
    assert!(matches!(
        extract_archive_to_staging(&archive, &staging),
        Err(AppError::Validation(_))
    ));
    assert!(!staging.exists());
    assert!(archive.exists());
}

#[test]
fn staging_cancellation_from_progress_removes_extracted_output() {
    let dir = TempDir::new().unwrap();
    let archive = create_test_zip(
        dir.path(),
        "cancelled.zip",
        &[("config.ini", b"[TextureOverride]\ndata")],
    );
    let staging = dir.path().join("staging");
    let token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let progress_token = token.clone();
    let channel = tauri::ipc::Channel::new(move |_| {
        progress_token.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    });
    let error = extract_archive_to_staging_with_options(
        &archive,
        &staging,
        StagingExtractOptions {
            cancel_token: Some(token),
            on_progress: Some(channel),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(error, AppError::Cancelled));
    assert!(!staging.exists());
    assert!(archive.exists());
}
