use crate::shared::errors::ScannerError;
use crate::modules::workspace::application::scanner::conflict::{
    detect_conflicts_with_roots, discover_runtime_ini_files, ConflictInfo,
};
use crate::modules::workspace::application::scanner::core::walker;
use std::path::Path;

/// Detect conflicts by scanning the entire mods folder for INI files.
pub fn detect_conflicts_in_folder_service(
    mods_path: &Path,
) -> Result<Vec<ConflictInfo>, ScannerError> {
    // Use walker to find all mod folders
    let candidates = walker::scan_mod_folders(mods_path)?;

    let mut all_inis = Vec::new();
    let mut mod_roots = Vec::new();
    for candidate in candidates {
        // Only check active mods
        if candidate.is_disabled {
            continue;
        }

        mod_roots.push(candidate.path.clone());
        for ini in discover_runtime_ini_files(&candidate.path) {
            all_inis.push((candidate.path.clone(), ini));
        }
    }

    Ok(detect_conflicts_with_roots(&all_inis, &mod_roots))
}
