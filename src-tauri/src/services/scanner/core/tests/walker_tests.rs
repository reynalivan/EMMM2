use super::*;
use std::fs;
use tempfile::TempDir;

fn create_test_mods_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");

    // Create mod folders
    fs::create_dir(dir.path().join("Raiden Mod")).unwrap();
    fs::create_dir(dir.path().join("DISABLED ayaka_skin")).unwrap();
    fs::create_dir(dir.path().join("unknown_123")).unwrap();

    // Create a non-dir file (should be ignored)
    fs::write(dir.path().join("readme.txt"), "test").unwrap();

    dir
}

// Covers: TC-2.3-01 — Scan mod folders listing
#[test]
fn test_scan_mod_folders_basic() {
    let dir = create_test_mods_dir();
    let candidates = scan_mod_folders(dir.path()).unwrap();

    assert_eq!(candidates.len(), 3);

    let names: Vec<&str> = candidates.iter().map(|c| c.raw_name.as_str()).collect();
    assert!(names.contains(&"Raiden Mod"));
    assert!(names.contains(&"DISABLED ayaka_skin"));
    assert!(names.contains(&"unknown_123"));
}

#[test]
fn test_scan_mod_folders_disabled_detection() {
    let dir = create_test_mods_dir();
    let candidates = scan_mod_folders(dir.path()).unwrap();

    let disabled = candidates
        .iter()
        .find(|c| c.raw_name == "DISABLED ayaka_skin")
        .unwrap();
    assert!(disabled.is_disabled);
    assert_eq!(disabled.display_name, "ayaka_skin");

    let enabled = candidates
        .iter()
        .find(|c| c.raw_name == "Raiden Mod")
        .unwrap();
    assert!(!enabled.is_disabled);
}

#[test]
fn test_scan_mod_folders_nonexistent() {
    let result = scan_mod_folders(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}

// Covers: Epic 2 §B.2 — Content scan with depth
#[test]
fn test_scan_folder_content() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("test_mod");
    fs::create_dir(&mod_dir).unwrap();

    // Create subfolder
    let sub = mod_dir.join("subfolder1");
    fs::create_dir(&sub).unwrap();

    // Create files
    fs::write(mod_dir.join("config.ini"), "[Section]\nkey=val").unwrap();
    fs::write(mod_dir.join("texture.dds"), "binary data").unwrap();
    fs::write(sub.join("nested.ini"), "[Nested]").unwrap();

    let content = scan_folder_content(&mod_dir, 3);

    assert_eq!(content.subfolder_names.len(), 1);
    assert!(content.subfolder_names.contains(&"subfolder1".to_string()));
    assert_eq!(content.ini_files.len(), 2);
    assert!(content.files.len() >= 3); // ini + dds + nested ini
}

// Covers: EC-2.05 — Zero-byte INI handling
#[test]
fn test_scan_folder_content_zero_byte_ini() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("zero_byte_mod");
    fs::create_dir(&mod_dir).unwrap();

    // Create zero-byte ini
    fs::write(mod_dir.join("empty.ini"), "").unwrap();

    let content = scan_folder_content(&mod_dir, 3);
    assert_eq!(content.ini_files.len(), 1);
    // Zero-byte file is still listed, matching logic handles it separately
}

// Covers: TC-2.1-01 — Archive detection
#[test]
fn test_detect_archives() {
    let dir = TempDir::new().unwrap();

    fs::write(dir.path().join("mod_pack.zip"), "fake zip").unwrap();
    fs::write(dir.path().join("textures.7z"), "fake 7z").unwrap();
    fs::write(dir.path().join("readme.txt"), "not archive").unwrap();
    fs::create_dir(dir.path().join("some_folder")).unwrap();

    let archives = detect_archives(dir.path()).unwrap();

    assert_eq!(archives.len(), 2);
    let exts: Vec<&str> = archives.iter().map(|a| a.extension.as_str()).collect();
    assert!(exts.contains(&"zip"));
    assert!(exts.contains(&"7z"));
}
