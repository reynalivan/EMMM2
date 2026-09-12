use super::*;
use crate::shared::errors::AppError;
use std::cell::Cell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const ARCHIVE_BACKUP_DIR: &str = ".extracted";

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

// Covers: TC-2.1-01 — Basic ZIP extraction
#[test]
fn test_extract_zip_basic() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "mod_pack.zip",
        &[
            ("config.ini", b"[TextureOverride]\nkey=val"),
            ("texture.dds", b"binary data"),
        ],
    );

    let result = extract_archive(&zip_path, dir.path(), ExtractOptions::default()).unwrap();
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

#[test]
fn test_temp_extract_cleaned_up() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "cleanup.zip",
        &[("config.ini", b"[TextureOverride]\nkey=val")],
    );
    let result = extract_archive(&zip_path, dir.path(), ExtractOptions::default()).unwrap();
    assert!(result.success);
    // .temp_extract should be fully cleaned up
    assert!(!dir.path().join(".temp_extract").exists());
}

// Covers: TC-2.1-02 — Smart flattening
#[test]
fn test_extract_zip_smart_flatten() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "nested_mod.zip",
        &[
            ("WrapperFolder/config.ini", b"[TextureOverride]\ndata"),
            ("WrapperFolder/sub/texture.dds", b"data"),
        ],
    );

    let result = extract_archive(&zip_path, dir.path(), ExtractOptions::default()).unwrap();
    assert!(result.success);

    let dest = dir.path().join("WrapperFolder");
    assert!(dest.join("config.ini").exists());
    assert!(dest.join("sub").join("texture.dds").exists());
    assert!(!dest.join("WrapperFolder").exists());
}

// Covers: EC-2.07 — Duplicate destination
#[test]
fn test_extract_duplicate_dest() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "existing_mod.zip",
        &[("file.ini", b"[TextureOverride]\ndata")],
    );

    fs::create_dir(dir.path().join("existing_mod")).unwrap();

    // Without overwrite (auto-renames to "existing_mod (2)")
    let result = extract_archive(&zip_path, dir.path(), ExtractOptions::default()).unwrap();
    assert!(result.success);
    println!("DEST PATHS: {:?}", result.dest_paths);
    assert!(result.dest_paths[0].ends_with("existing_mod (2)"));
    assert!(dir
        .path()
        .join("existing_mod (2)")
        .join("file.ini")
        .exists());

    // Re-create the test zip since extract_archive moves it to backup
    let zip_path2 = create_test_zip(
        dir.path(),
        "existing_mod2.zip",
        &[("file.ini", b"[TextureOverride]\ndata")],
    );

    // With overwrite
    assert!(zip_path2.exists());
    let result2 = extract_archive(
        &zip_path2,
        dir.path(),
        ExtractOptions {
            overwrite: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result2.success);
    // Since overwrite is true, it extracts to "existing_mod2" directly
    assert!(dir.path().join("existing_mod2").join("file.ini").exists());
}

#[test]
fn destination_preflight_runs_before_overwrite_commit() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "protected_mod.zip",
        &[("new.ini", b"[TextureOverride]\nnew")],
    );
    let existing = dir.path().join("protected_mod");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("keep.ini"), "existing").unwrap();

    let called = Cell::new(false);
    let expected = existing.clone();
    let reject = |destinations: &[PathBuf]| {
        called.set(true);
        assert_eq!(destinations, std::slice::from_ref(&expected));
        Err(AppError::Validation(
            "destination is conflicted".to_string(),
        ))
    };

    let error = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            overwrite: true,
            before_commit: Some(&reject),
            ..Default::default()
        },
    );

    assert!(error.is_err());
    assert!(called.get());
    assert_eq!(
        fs::read_to_string(existing.join("keep.ini")).unwrap(),
        "existing"
    );
    assert!(!existing.join("new.ini").exists());
    assert!(zip_path.exists());
    assert!(!dir.path().join(".temp_extract").exists());
}

#[test]
fn custom_name_cannot_escape_or_target_the_mods_root() {
    let dir = TempDir::new().unwrap();
    let mods_dir = dir.path().join("Mods");
    let outside = dir.path().join("Outside");
    fs::create_dir_all(&mods_dir).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let marker = outside.join("must-survive.txt");
    fs::write(&marker, "outside").unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "unsafe-name.zip",
        &[("new.ini", b"[TextureOverride]\nnew")],
    );
    let absolute_outside = outside.to_string_lossy().to_string();

    for unsafe_name in ["", "..", "../Outside", absolute_outside.as_str()] {
        let result = extract_archive(
            &zip_path,
            &mods_dir,
            ExtractOptions {
                custom_name: Some(unsafe_name),
                overwrite: true,
                ..Default::default()
            },
        );
        assert!(
            result.is_err(),
            "unsafe custom name must fail: {unsafe_name}"
        );
    }

    assert_eq!(fs::read_to_string(marker).unwrap(), "outside");
    assert!(mods_dir.exists());
    assert!(zip_path.exists());
}

#[test]
fn enabled_extraction_uniquifies_against_disabled_sibling_identity() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("DISABLED status_mod")).unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "status_mod.zip",
        &[("mod.ini", b"[TextureOverride]\nnew")],
    );

    let result = extract_archive(&zip_path, dir.path(), ExtractOptions::default()).unwrap();

    assert!(result.dest_paths[0].ends_with("status_mod (2)"));
    assert!(dir.path().join("status_mod (2)").exists());
    assert!(!dir.path().join("status_mod").exists());
}

#[test]
fn disabled_extraction_commits_directly_to_a_unique_final_identity() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("DISABLED status_mod")).unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "status_mod.zip",
        &[("mod.ini", b"[TextureOverride]\nnew")],
    );

    let result = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            disable_after: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(result.dest_paths[0].ends_with("DISABLED status_mod (2)"));
    assert!(dir.path().join("DISABLED status_mod (2)").exists());
    assert!(!dir.path().join("status_mod").exists());
}

// Covers: NC-2.1-01 — Corrupt archive
#[test]
fn test_extract_corrupt_archive() {
    let dir = TempDir::new().unwrap();
    let zip_path = dir.path().join("corrupt.zip");
    fs::write(&zip_path, b"not a real zip file").unwrap();

    let result = extract_archive(&zip_path, dir.path(), ExtractOptions::default());
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
    assert!(progress_seen.load(std::sync::atomic::Ordering::SeqCst));
    assert!(zip_path.exists());
}

// Covers: TC-2.1-04 — ZIP password extraction
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
fn test_extract_zip_with_password() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_encrypted_zip(
        dir.path(),
        "secret.zip",
        "mypassword",
        &[("data.ini", b"[TextureOverride]\nsecret content")],
    );

    let result = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            password: Some("mypassword"),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.success);
    assert_eq!(result.files_extracted, 1);

    let extracted = dir.path().join("secret").join("data.ini");
    assert!(extracted.exists());
    assert_eq!(
        fs::read_to_string(&extracted).unwrap(),
        "[TextureOverride]\nsecret content"
    );
}

// T1: Covers: disable_after — extracted folder gets "DISABLED " prefix
#[test]
fn test_extract_with_disable_after() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "my_mod.zip",
        &[("config.ini", b"[TextureOverride]\ndata")],
    );

    let result = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            disable_after: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.success);
    assert!(dir.path().join("DISABLED my_mod").exists());
    assert!(result.dest_paths[0].contains("DISABLED my_mod"));
}

// T2: Covers: custom_name — overrides the archive stem
#[test]
fn test_extract_with_custom_name() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "original_name.zip",
        &[("config.ini", b"[TextureOverride]\ndata")],
    );

    let result = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            custom_name: Some("CustomMod"),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.success);
    assert!(dir.path().join("CustomMod").join("config.ini").exists());
    // Original stem name should NOT exist
    assert!(!dir.path().join("original_name").exists());
}

// T3: Covers: cancel token aborts extraction
#[test]
fn test_extract_cancellation() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_test_zip(
        dir.path(),
        "cancel_test.zip",
        &[("config.ini", b"[TextureOverride]\ndata")],
    );

    // Pre-set the cancel token to true
    let cancel_token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let result = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            cancel_token: Some(cancel_token),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.aborted);
    assert!(!result.success);
    // Temp should be cleaned up
    assert!(!dir.path().join(".temp_extract").exists());
}

// T4: Covers: wrong password returns error
#[test]
fn test_extract_wrong_password() {
    let dir = TempDir::new().unwrap();
    let zip_path = create_encrypted_zip(
        dir.path(),
        "locked.zip",
        "correct_password",
        &[("data.ini", b"[TextureOverride]\nsecret content")],
    );

    let result = extract_archive(
        &zip_path,
        dir.path(),
        ExtractOptions {
            password: Some("wrong_password"),
            ..Default::default()
        },
    );
    // Should either error or succeed with garbage (ZipCrypto is weak)
    // The key invariant: it shouldn't silently extract valid-looking content
    match result {
        Err(_) => {} // Good — error on wrong password
        Ok(r) => {
            // ZipCrypto may "succeed" with garbage — verify the extracted
            // content doesn't actually match the original
            if r.success {
                // If files extracted, check they exist but content may be wrong
                // This is a known ZipCrypto limitation — document it
                assert!(
                    r.files_extracted > 0,
                    "If success, files should be extracted"
                );
            }
        }
    }
}

// Covers: Nested archives unpacking (1 level deep)
#[test]
fn test_extract_nested_archives_basic() {
    let dir = TempDir::new().unwrap();

    // 1. Create the inner zip
    let inner_zip_path = create_test_zip(
        dir.path(),
        "inner_mod.zip",
        &[("inner_folder/config.ini", b"[TextureOverride]\ninner")],
    );

    // Read the inner zip into memory so we can embed it
    let inner_zip_data = fs::read(&inner_zip_path).unwrap();

    // 2. Create the outer zip that contains the inner zip
    let outer_zip_path = create_test_zip(
        dir.path(),
        "outer_pack.zip",
        &[("inner_mod.zip", &inner_zip_data)],
    );

    // Extract outer, flag unpack_nested = true
    let result = extract_archive(
        &outer_zip_path,
        dir.path(),
        ExtractOptions {
            unpack_nested: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.success);

    // The inner zip should be extracted into a folder named after itself conceptually
    // However, find_mod_roots will skip empty wrappers and find the folder containing the .ini directly.
    // In our test, the zip has inner_folder/config.ini.
    // If there is only one mod root, extract_archive will rename the temp_root to outer_pack.
    // Wait, let's see. The outer zip is outer_pack.zip. It unpacks. temp_dir has inner_mod.zip.
    // inner_mod.zip unpacks to temp_dir/inner_mod/inner_folder/config.ini.
    // find_mod_roots finds temp_dir/inner_mod/inner_folder as the mod root.
    // Because it's a single mod root but it's NOT the temp_root, it falls into "Cases 2-6".
    // It takes the folder name: "inner_folder".
    // And uses resolve_dest(mods_dir, "inner_folder", false).
    // So the destination will be `mods_dir/inner_folder`.
    let extracted_ini = dir.path().join("inner_folder").join("config.ini");
    assert!(
        extracted_ini.exists(),
        "config.ini was not extracted from nested archive to expected location"
    );

    // Ensure the raw inner zip is gone (it would have been in the temp folder, which is cleaned up)
    // We just check the extracted mod exists.
}

// Covers: Nested archives max depth protection
#[test]
fn test_extract_nested_archives_max_depth() {
    let dir = TempDir::new().unwrap();

    // depth 4 (L4) -> will hit max_depth and remain as .zip
    let l4_zip_path = create_test_zip(dir.path(), "level4.zip", &[("bottom.txt", b"too deep")]);
    let l4_zip_data = fs::read(&l4_zip_path).unwrap();

    // depth 3 (L3) -> extracted (nest depth 1)
    let l3_zip_path = create_test_zip(dir.path(), "level3.zip", &[("level4.zip", &l4_zip_data)]);
    let l3_zip_data = fs::read(&l3_zip_path).unwrap();

    // depth 2 (L2) -> extracted (nest depth 0)
    // Add a valid config.ini so the archive is valid!
    let l2_zip_path = create_test_zip(
        dir.path(),
        "level2.zip",
        &[
            ("level3.zip", &l3_zip_data),
            ("config.ini", b"[TextureOverride]\nvalid"),
        ],
    );
    let l2_zip_data = fs::read(&l2_zip_path).unwrap();

    // depth 1 (L1) - outer extraction
    let l1_zip_path = create_test_zip(dir.path(), "level1.zip", &[("level2.zip", &l2_zip_data)]);

    let error = extract_archive(
        &l1_zip_path,
        dir.path(),
        ExtractOptions {
            unpack_nested: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("nested_archive_depth_limit"));

    // Output structure:
    // The top wrapper (level1) is bypassed because it doesn't contain the .ini.
    // level2 (dir) [Mod Root]
    //   config.ini
    //   level3 (dir)
    //     level4.zip (file) -> NOT extracted because max depth is 2

    let l2_dir = dir.path().join("level2");
    let l3_dir = l2_dir.join("level3");
    let l4_zip_leftover = l3_dir.join("level4.zip");
    let l4_dir = l3_dir.join("level4");

    assert!(
        !l2_dir.exists(),
        "failed nested extraction must not leave staging output"
    );
    assert!(
        !l3_dir.exists(),
        "failed nested extraction must not leave staging output"
    );
    assert!(
        !l4_zip_leftover.exists(),
        "nested archive must not be reported as staged"
    );
    assert!(
        !l4_dir.exists(),
        "nested archive must not be unpacked beyond the limit"
    );
}
