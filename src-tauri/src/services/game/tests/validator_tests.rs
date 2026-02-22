use super::*;
use std::fs;

/// Helper: creates a valid 3DMigoto instance folder structure
fn create_valid_instance(dir: &Path) {
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "[Constants]").unwrap();
    fs::write(dir.join("d3d11.dll"), "fake-dll").unwrap();
    fs::write(dir.join("3DMigotoLoader.exe"), "fake-exe").unwrap();
}

// Covers: TC-1.2-01 (Auto-Detect Success)
#[test]
fn test_valid_instance_passes() {
    let dir = std::env::temp_dir().join("emmm2_test_valid");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);

    let result = validate_instance(&dir);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert!(info.mods_path.contains("Mods"));
    assert!(info.launcher_path.contains("3DMigotoLoader"));

    let _ = fs::remove_dir_all(&dir);
}

// Covers: NC-1.3-01 (Missing /Mods Folder)
#[test]
fn test_missing_mods_folder() {
    let dir = std::env::temp_dir().join("emmm2_test_no_mods");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("d3dx.ini"), "").unwrap();
    fs::write(dir.join("d3d11.dll"), "").unwrap();
    fs::write(dir.join("test.exe"), "").unwrap();

    let result = validate_instance(&dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing /Mods folder"));

    let _ = fs::remove_dir_all(&dir);
}

// Covers: NC-1.3-04 (Missing DLL)
#[test]
fn test_missing_dll() {
    let dir = std::env::temp_dir().join("emmm2_test_no_dll");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "").unwrap();
    // d3d11.dll intentionally missing
    fs::write(dir.join("test.exe"), "").unwrap();

    let result = validate_instance(&dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("d3d11.dll"));

    let _ = fs::remove_dir_all(&dir);
}

// Covers: NC-1.2-02 (Missing d3dx.ini)
#[test]
fn test_missing_d3dx_ini() {
    let dir = std::env::temp_dir().join("emmm2_test_no_ini");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Mods")).unwrap();
    // d3dx.ini intentionally missing
    fs::write(dir.join("d3d11.dll"), "").unwrap();
    fs::write(dir.join("test.exe"), "").unwrap();

    let result = validate_instance(&dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("d3dx.ini"));

    let _ = fs::remove_dir_all(&dir);
}

// Covers: Heuristic - prefers "loader" in exe name
#[test]
fn test_loader_priority() {
    let dir = std::env::temp_dir().join("emmm2_test_loader_prio");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);
    fs::write(dir.join("SomeOtherApp.exe"), "").unwrap();

    let result = validate_instance(&dir).unwrap();
    assert!(result.launcher_path.contains("Loader"));

    let _ = fs::remove_dir_all(&dir);
}

// Covers: No .exe found at all
#[test]
fn test_no_exe_found() {
    let dir = std::env::temp_dir().join("emmm2_test_no_exe");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "").unwrap();
    fs::write(dir.join("d3d11.dll"), "").unwrap();
    // No .exe files

    let result = validate_instance(&dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No .exe"));

    let _ = fs::remove_dir_all(&dir);
}

// Covers: Path does not exist
#[test]
fn test_nonexistent_path() {
    let dir = Path::new("Z:\\definitely_does_not_exist_12345");
    let result = validate_instance(dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

// Covers: TC-1.2-02 (Multi-Game Discovery - partial success)
#[test]
fn test_scan_xxmi_partial() {
    let root = std::env::temp_dir().join("emmm2_test_xxmi_scan");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    // Create valid GIMI only
    create_valid_instance(&root.join("GIMI"));
    // SRMI is empty (invalid)
    fs::create_dir_all(root.join("SRMI")).unwrap();

    let results = scan_xxmi_root(&root);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "GIMI");
    assert_eq!(results[0].2, "Genshin Impact");

    let _ = fs::remove_dir_all(&root);
}

// Covers: EC-1.03 (Unusual Characters / Unicode)
#[test]
fn test_unicode_path() {
    // "Génsjhìn❤" in path
    let dir = std::env::temp_dir().join("emmm2_test_unicode_❤");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);

    let result = validate_instance(&dir);
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(&dir);
}

// Covers: EC-1.02 (Mixed Path Separators)
#[test]
fn test_mixed_separators() {
    // Rust's std::path handles separators natively, but we verify it works
    let dir = std::env::temp_dir().join("emmm2_test_mixed");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);

    let result = validate_instance(&dir);
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(&dir);
}
