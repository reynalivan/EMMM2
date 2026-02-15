use crate::database::models::GameInfo;
use std::path::Path;

/// Validates a folder as a valid 3DMigoto game instance.
///
/// Checks for:
/// 1. `/Mods` subfolder (mandatory)
/// 2. `d3dx.ini` config file (3DMigoto fingerprint)
/// 3. `d3d11.dll` injection DLL
/// 4. At least one `.exe` (prefers names containing "loader")
pub fn validate_instance(path: &Path) -> Result<GameInfo, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    // RULE 1: /Mods folder is mandatory
    let mods_path = path.join("Mods");
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err("Missing /Mods folder".to_string());
    }

    // RULE 2: Core 3DMigoto files
    if !path.join("d3dx.ini").exists() {
        return Err("Missing core file: d3dx.ini".to_string());
    }
    if !path.join("d3d11.dll").exists() {
        return Err("Missing core file: d3d11.dll".to_string());
    }

    // RULE 3: Find launcher .exe (prefer names containing "loader")
    let exe_files: Vec<_> = std::fs::read_dir(path)
        .map_err(|e| format!("Cannot read directory: {e}"))?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
        })
        .map(|e| e.path())
        .collect();

    if exe_files.is_empty() {
        return Err("No .exe launcher found".to_string());
    }

    let launcher = exe_files
        .iter()
        .find(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains("loader")
        })
        .unwrap_or(&exe_files[0])
        .clone();

    Ok(GameInfo {
        path: path.to_string_lossy().to_string(),
        launcher_path: launcher.to_string_lossy().to_string(),
        mods_path: mods_path.to_string_lossy().to_string(),
    })
}

/// Known XXMI subfolder names and their display names
pub const XXMI_TARGETS: &[(&str, &str)] = &[
    ("GIMI", "Genshin Impact"),
    ("SRMI", "Honkai Star Rail"),
    ("WWMI", "Wuthering Waves"),
    ("ZZMI", "Zenless Zone Zero"),
    ("EFMI", "Arknight Endfield"),
];

/// Scans an XXMI root folder for known game subfolders
pub fn scan_xxmi_root(root: &Path) -> Vec<(GameInfo, &'static str, &'static str)> {
    XXMI_TARGETS
        .iter()
        .filter_map(|(folder, name)| {
            let full = root.join(folder);
            validate_instance(&full)
                .ok()
                .map(|info| (info, *folder, *name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
}
