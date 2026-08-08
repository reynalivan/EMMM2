use crate::domain::errors::AppError;
use crate::domain::models::{GameInfo, GameType};
use std::path::{Path, PathBuf};

/// Files 3DMigoto needs in the game root. Missing ones are soft warnings.
const CORE_FILES: [&str; 2] = ["d3dx.ini", "d3d11.dll"];

/// A game instance discovered under an XXMI root.
pub struct DetectedGame {
    pub info: GameInfo,
    pub warnings: Vec<String>,
    pub game_type: GameType,
}

/// Validates a folder as a valid 3DMigoto game instance.
///
/// Instead of failing hard on missing optional files, this function:
/// 1. Auto-corrects if the user pointed at the `/Mods` subfolder — climbs up to the parent.
/// 2. Treats `/Mods`, `d3dx.ini`, `d3d11.dll`, and `.exe` as soft warnings, not hard errors.
///
/// Returns `(GameInfo, Vec<String>)` where warnings are displayed in the UI.
pub fn validate_instance(raw_path: &Path) -> Result<(GameInfo, Vec<String>), AppError> {
    if !raw_path.exists() {
        return Err(AppError::Internal(format!(
            "Path does not exist: {}",
            raw_path.display()
        )));
    }

    let mut warnings: Vec<String> = Vec::new();

    // SMART: If user selected the /Mods folder itself, silently climb up to the parent
    let selected_mods_folder = raw_path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("mods"));
    let path: PathBuf = if selected_mods_folder {
        log::debug!("Smart path correction: user selected /Mods, resolved to parent.");
        raw_path
            .parent()
            .ok_or_else(|| {
                AppError::Internal("Cannot resolve parent of selected 'Mods' folder.".to_string())
            })?
            .to_path_buf()
    } else {
        raw_path.to_path_buf()
    };

    // RULE 1: /Mods folder (soft — warn if missing)
    let mods_path = path.join("Mods");
    if !mods_path.is_dir() {
        warnings.push(
            "Missing /Mods folder. You may need to create it manually before installing mods."
                .to_string(),
        );
    }

    // RULE 2: Core 3DMigoto files (soft — warn if missing)
    for core_file in CORE_FILES {
        if !path.join(core_file).exists() {
            warnings.push(format!(
                "Missing core file: {core_file} (3DMigoto may not be installed correctly here)."
            ));
        }
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
        mods_path: mods_path.to_string_lossy().to_string(),
    };

    Ok((info, warnings))
}

/// Finds the most appropriate launcher .exe in the given directory.
/// Prefers filenames containing "loader", falls back to any .exe.
fn find_launcher(path: &Path) -> Option<PathBuf> {
    let exe_files: Vec<PathBuf> = std::fs::read_dir(path)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
        })
        .collect();

    exe_files
        .iter()
        .find(|path| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains("loader")
        })
        .or(exe_files.first())
        .cloned()
}

/// Scans an XXMI root folder for known game subfolders. The subfolder name is
/// the game's own code, so the roster comes from `GameType` rather than a
/// second table kept in sync by hand.
pub fn scan_xxmi_root(root: &Path) -> Vec<DetectedGame> {
    GameType::ALL
        .into_iter()
        .filter_map(|game_type| {
            let full = root.join(game_type.to_string());
            validate_instance(&full)
                .ok()
                .map(|(info, warnings)| DetectedGame {
                    info,
                    warnings,
                    game_type,
                })
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/validator_tests.rs"]
mod tests;
