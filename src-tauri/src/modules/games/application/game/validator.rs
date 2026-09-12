use crate::modules::games::domain::models::{GameInfo, GameType, LaunchMode};
use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};

/// Files 3DMigoto needs in the game root. Missing ones are soft warnings.
const CORE_FILES: [&str; 2] = ["d3dx.ini", "d3d11.dll"];

/// A game instance discovered under an XXMI root.
pub struct DetectedGame {
    pub info: GameInfo,
    pub warnings: Vec<String>,
    pub game_type: GameType,
    pub launch_mode: LaunchMode,
    pub xxmi_launcher_exe: Option<PathBuf>,
}

/// Validates a folder as a valid 3DMigoto game instance.
///
/// Instead of failing hard on missing optional files, this function:
/// 1. Resolves an instance root, its `/Mods` folder, or any nested Mods subfolder.
/// 2. Treats `/Mods`, `d3dx.ini`, `d3d11.dll`, and `.exe` as soft warnings, not hard errors.
///
/// Returns `(GameInfo, Vec<String>)` where warnings are displayed in the UI.
pub fn validate_instance(raw_path: &Path) -> Result<(GameInfo, Vec<String>), AppError> {
    validate_instance_with_launcher_requirement(raw_path, true)
}

fn validate_instance_with_launcher_requirement(
    raw_path: &Path,
    require_standalone_launcher: bool,
) -> Result<(GameInfo, Vec<String>), AppError> {
    if !raw_path.exists() {
        return Err(AppError::Internal(format!(
            "Path does not exist: {}",
            raw_path.display()
        )));
    }

    let mut warnings: Vec<String> = Vec::new();

    let (path, mods_path) = resolve_instance_paths(raw_path)?;

    // RULE 1: /Mods folder (soft — warn if missing)
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
    let launcher_path = find_launcher(&path).map(|launcher| launcher.to_string_lossy().to_string());
    if require_standalone_launcher && launcher_path.is_none() {
        warnings.push(
            "No standalone launcher found. Configure a 3DMigoto loader before using Play."
                .to_string(),
        );
    }

    let info = GameInfo {
        path: path.to_string_lossy().to_string(),
        launcher_path,
        mods_path: mods_path.to_string_lossy().to_string(),
    };

    Ok((info, warnings))
}

fn resolve_instance_paths(raw_path: &Path) -> Result<(PathBuf, PathBuf), AppError> {
    if let Some(mods_root) = raw_path.ancestors().find(|candidate| {
        candidate
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("mods"))
    }) {
        let instance_root = mods_root.parent().ok_or_else(|| {
            AppError::Internal("Cannot resolve parent of selected 'Mods' folder.".to_string())
        })?;
        log::debug!(
            "Smart path correction: selected Mods path {}, instance root {}.",
            raw_path.display(),
            instance_root.display()
        );
        return Ok((instance_root.to_path_buf(), raw_path.to_path_buf()));
    }

    let nested_mods_path = raw_path.join("Mods");
    let looks_like_instance_root = nested_mods_path.is_dir()
        || CORE_FILES
            .iter()
            .any(|file_name| raw_path.join(file_name).exists());
    if looks_like_instance_root {
        return Ok((raw_path.to_path_buf(), nested_mods_path));
    }

    Ok((raw_path.to_path_buf(), raw_path.to_path_buf()))
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
    let xxmi_launcher_exe = root.join("Resources").join("Bin").join("XXMI Launcher.exe");
    if !is_executable_file(&xxmi_launcher_exe) {
        return Vec::new();
    }

    GameType::ALL
        .into_iter()
        .filter_map(|game_type| {
            let full = root.join(game_type.to_string());
            validate_instance_with_launcher_requirement(&full, false)
                .ok()
                .map(|(mut info, warnings)| {
                    info.launcher_path = Some(xxmi_launcher_exe.to_string_lossy().to_string());
                    DetectedGame {
                        info,
                        warnings,
                        game_type,
                        launch_mode: LaunchMode::XxmiManaged,
                        xxmi_launcher_exe: Some(xxmi_launcher_exe.clone()),
                    }
                })
        })
        .collect()
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

#[cfg(test)]
#[path = "tests/validator_tests.rs"]
mod tests;
