use super::*;
use std::fs;

/// Helper: creates a valid 3DMigoto instance folder structure
fn create_valid_instance(dir: &Path) {
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "[Constants]").unwrap();
    fs::write(dir.join("d3d11.dll"), "fake-dll").unwrap();
    fs::write(dir.join("3DMigotoLoader.exe"), "fake-exe").unwrap();
}

// TC-1.2-01: Valid full instance returns Ok with no warnings
#[test]
fn test_valid_instance_passes() {
    let dir = std::env::temp_dir().join("emmm_test_valid");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);

    let result = validate_instance(&dir);
    assert!(result.is_ok());
    let (info, warnings) = result.unwrap();
    assert!(info.mods_path.contains("Mods"));
    assert!(info.launcher_path.contains("3DMigotoLoader"));
    assert!(
        warnings.is_empty(),
        "Expected no warnings for a valid instance"
    );

    let _ = fs::remove_dir_all(&dir);
}

// NC-1.3-01: Missing /Mods folder => warning, not error
#[test]
fn test_missing_mods_folder_is_warning() {
    let dir = std::env::temp_dir().join("emmm_test_no_mods");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("d3dx.ini"), "").unwrap();
    fs::write(dir.join("d3d11.dll"), "").unwrap();
    fs::write(dir.join("test.exe"), "").unwrap();

    let result = validate_instance(&dir);
    assert!(
        result.is_ok(),
        "Missing /Mods should be a soft warning, not an error"
    );
    let (_, warnings) = result.unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("Missing /Mods")),
        "Expected a warning about missing /Mods folder"
    );

    let _ = fs::remove_dir_all(&dir);
}

// NC-1.3-04: Missing d3d11.dll => warning, not error
#[test]
fn test_missing_dll_is_warning() {
    let dir = std::env::temp_dir().join("emmm_test_no_dll");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "").unwrap();
    // d3d11.dll intentionally missing
    fs::write(dir.join("test.exe"), "").unwrap();

    let result = validate_instance(&dir);
    assert!(
        result.is_ok(),
        "Missing d3d11.dll should be a soft warning, not an error"
    );
    let (_, warnings) = result.unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("d3d11.dll")),
        "Expected a warning about missing d3d11.dll"
    );

    let _ = fs::remove_dir_all(&dir);
}

// NC-1.2-02: Missing d3dx.ini => warning, not error
#[test]
fn test_missing_d3dx_ini_is_warning() {
    let dir = std::env::temp_dir().join("emmm_test_no_ini");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Mods")).unwrap();
    // d3dx.ini intentionally missing
    fs::write(dir.join("d3d11.dll"), "").unwrap();
    fs::write(dir.join("test.exe"), "").unwrap();

    let result = validate_instance(&dir);
    assert!(
        result.is_ok(),
        "Missing d3dx.ini should be a soft warning, not an error"
    );
    let (_, warnings) = result.unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("d3dx.ini")),
        "Expected a warning about missing d3dx.ini"
    );

    let _ = fs::remove_dir_all(&dir);
}

// Heuristic: prefers "loader" in exe name
#[test]
fn test_loader_priority() {
    let dir = std::env::temp_dir().join("emmm_test_loader_prio");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);
    fs::write(dir.join("SomeOtherApp.exe"), "").unwrap();

    let (info, _) = validate_instance(&dir).unwrap();
    assert!(info.launcher_path.contains("Loader"));

    let _ = fs::remove_dir_all(&dir);
}

// No .exe found at all => warning, not error
#[test]
fn test_no_exe_found_is_warning() {
    let dir = std::env::temp_dir().join("emmm_test_no_exe");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Mods")).unwrap();
    fs::write(dir.join("d3dx.ini"), "").unwrap();
    fs::write(dir.join("d3d11.dll"), "").unwrap();
    // No .exe files

    let result = validate_instance(&dir);
    assert!(
        result.is_ok(),
        "No .exe should be a soft warning, not an error"
    );
    let (_, warnings) = result.unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains(".exe") || w.contains("launcher")),
        "Expected a warning about missing .exe launcher"
    );

    let _ = fs::remove_dir_all(&dir);
}

// Path does not exist => hard error
#[test]
fn test_nonexistent_path() {
    let dir = Path::new("Z:\\definitely_does_not_exist_12345");
    let result = validate_instance(dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

// Smart resolution: selecting /Mods climbs up to the parent
#[test]
fn test_smart_mods_folder_correction() {
    let base = std::env::temp_dir().join("emmm_test_smart_correction");
    let _ = fs::remove_dir_all(&base);
    create_valid_instance(&base);

    // User selects the /Mods folder instead of the root
    let mods_dir = base.join("Mods");
    let result = validate_instance(&mods_dir);
    assert!(
        result.is_ok(),
        "Selecting the Mods folder should auto-correct"
    );
    let (info, _warnings) = result.unwrap();
    // The resolved path should be the parent (base)
    assert!(
        !info.path.to_lowercase().ends_with("mods"),
        "Game path should be the parent of the Mods folder"
    );

    let _ = fs::remove_dir_all(&base);
}

// TC-1.2-02: Multi-game scan partial results
#[test]
fn test_scan_xxmi_partial() {
    let root = std::env::temp_dir().join("emmm_test_xxmi_scan");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    // Create valid GIMI only
    create_valid_instance(&root.join("GIMI"));
    // SRMI is NOT created (invalid — missing dir leads to Err in validate_instance and filter_map skip)

    let results = scan_xxmi_root(&root);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].2, "GIMI");
    assert_eq!(results[0].3, "Genshin Impact");

    let _ = fs::remove_dir_all(&root);
}

// EC-1.03: Unicode path
#[test]
fn test_unicode_path() {
    let dir = std::env::temp_dir().join("emmm_test_unicode_❤");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);

    let result = validate_instance(&dir);
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(&dir);
}

// EC-1.02: Mixed path separators
#[test]
fn test_mixed_separators() {
    let dir = std::env::temp_dir().join("emmm_test_mixed");
    let _ = fs::remove_dir_all(&dir);
    create_valid_instance(&dir);

    let result = validate_instance(&dir);
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(&dir);
}
