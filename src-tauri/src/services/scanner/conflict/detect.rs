use crate::services::scanner::conflict::{detect_conflicts, ConflictInfo};
use crate::services::scanner::core::walker;
use std::path::Path;

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
