use crate::database::models::GameInfo;
use std::path::{Path, PathBuf};

/// Smart result type: (resolved path, warnings)
pub type ValidationResult = (GameInfo, Vec<String>);

/// Validates a folder as a valid 3DMigoto game instance.
///
/// Instead of failing hard on missing optional files, this function:
/// 1. Auto-corrects if the user pointed at the `/Mods` subfolder — climbs up to the parent.
/// 2. Treats `/Mods`, `d3dx.ini`, `d3d11.dll`, and `.exe` as soft warnings, not hard errors.
///
/// Returns `(GameInfo, Vec<String>)` where warnings are displayed in the UI.
pub fn validate_instance(raw_path: &Path) -> Result<ValidationResult, String> {
    if !raw_path.exists() {
        return Err(format!("Path does not exist: {}", raw_path.display()));
    }

    let mut warnings: Vec<String> = Vec::new();

    // SMART: If user selected the /Mods folder itself, silently climb up to the parent
    let path: PathBuf = {
        let folder_name = raw_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if folder_name == "mods" {
            let parent = raw_path
                .parent()
                .ok_or_else(|| "Cannot resolve parent of selected 'Mods' folder.".to_string())?;
            log::debug!("Smart path correction: user selected /Mods, resolved to parent.");
            parent.to_path_buf()
        } else {
            raw_path.to_path_buf()
        }
    };

    // RULE 1: /Mods folder (soft — warn if missing)
    let mods_path = path.join("Mods");
    let resolved_mods_path = if mods_path.exists() && mods_path.is_dir() {
        mods_path.to_string_lossy().to_string()
    } else {
        warnings.push(
            "Missing /Mods folder. You may need to create it manually before installing mods."
                .to_string(),
        );
        mods_path.to_string_lossy().to_string()
    };

    // RULE 2: Core 3DMigoto files (soft — warn if missing)
    if !path.join("d3dx.ini").exists() {
        warnings.push(
            "Missing core file: d3dx.ini (3DMigoto may not be installed correctly here)."
                .to_string(),
        );
    }
    if !path.join("d3d11.dll").exists() {
        warnings.push(
            "Missing core file: d3d11.dll (3DMigoto may not be installed correctly here)."
                .to_string(),
        );
    }

    // RULE 3: Find launcher .exe (soft — warn if missing)
    let launcher_path = match find_launcher(&path) {
        Some(launcher) => launcher.to_string_lossy().to_string(),
        None => {
            warnings.push(
                "No .exe launcher found. Auto-launch will not work until a launcher is configured."
                    .to_string(),
            );
            path.to_string_lossy().to_string() // fallback to the game folder itself
        }
    };

    let info = GameInfo {
        path: path.to_string_lossy().to_string(),
        launcher_path,
        mods_path: resolved_mods_path,
    };

    Ok((info, warnings))
}

/// Finds the most appropriate launcher .exe in the given directory.
/// Prefers filenames containing "loader", falls back to any .exe.
fn find_launcher(path: &Path) -> Option<PathBuf> {
    let exe_files: Vec<PathBuf> = std::fs::read_dir(path)
        .ok()?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
        })
        .map(|e| e.path())
        .collect();

    if exe_files.is_empty() {
        return None;
    }

    let preferred = exe_files.iter().find(|p| {
        p.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .contains("loader")
    });

    Some(preferred.unwrap_or(&exe_files[0]).clone())
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
pub fn scan_xxmi_root(root: &Path) -> Vec<(GameInfo, Vec<String>, &'static str, &'static str)> {
    XXMI_TARGETS
        .iter()
        .filter_map(|(folder, name)| {
            let full = root.join(folder);
            validate_instance(&full)
                .ok()
                .map(|(info, warnings)| (info, warnings, *folder, *name))
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/validator_tests.rs"]
mod tests;
