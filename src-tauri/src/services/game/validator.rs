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
#[path = "tests/validator_tests.rs"]
mod tests;
