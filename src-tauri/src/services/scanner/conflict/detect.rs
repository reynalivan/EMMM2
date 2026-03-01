use crate::services::scanner::conflict::{detect_conflicts, ConflictInfo};
use crate::services::scanner::core::walker;
use std::path::{Path, PathBuf};

/// Detect conflicts by scanning the entire mods folder for INI files.
pub fn detect_conflicts_in_folder_service(mods_path: &Path) -> Result<Vec<ConflictInfo>, String> {
    // Use walker to find all mod folders
    let candidates = walker::scan_mod_folders(mods_path)?;

    let mut all_inis = Vec::new();
    for candidate in candidates {
        // Only check active mods
        if candidate.is_disabled {
            continue;
        }

        let content = walker::scan_folder_content(&candidate.path, 3); // Depth 3 per Epic 2
        for ini in content.ini_files {
            all_inis.push((candidate.path.clone(), ini));
        }
    }

    Ok(detect_conflicts(&all_inis))
}

/// Detect conflicts specifically involving a target mod.
pub fn detect_conflicts_for_mod_service(
    target: &Path,
    mods_dir: &Path,
) -> Result<Vec<ConflictInfo>, String> {
    // Collect all INIs from the active mods directory
    let ini_files = collect_ini_files(mods_dir)?;

    // Run conflict detection (CPU-bound, but fast for typical mod counts)
    let conflicts = detect_conflicts(&ini_files);

    // Filter to only conflicts involving the target mod
    let target_str = target.to_string_lossy().to_string();
    let relevant: Vec<ConflictInfo> = conflicts
        .into_iter()
        .filter(|c| c.mod_paths.iter().any(|p| p.starts_with(&target_str)))
        .collect();

    Ok(relevant)
}

/// Walk a directory and collect all `.ini` files from immediate subdirectories.
pub fn collect_ini_files(mods_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut ini_files = Vec::new();

    let entries =
        std::fs::read_dir(mods_dir).map_err(|e| format!("Failed to read mods dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Skip disabled mods (they won't conflict in-game)
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.to_uppercase().starts_with("DISABLED") {
            continue;
        }
        // Walk this mod folder for .ini files
        walk_ini_recursive(&path, &path, &mut ini_files);
    }

    Ok(ini_files)
}

/// Recursively collect `.ini` files from a directory.
fn walk_ini_recursive(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, PathBuf)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_ini_recursive(root, &path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"))
        {
            out.push((root.to_path_buf(), path));
        }
    }
}
