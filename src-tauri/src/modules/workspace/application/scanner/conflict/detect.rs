use crate::modules::workspace::application::scanner::conflict::hash_scan::detect_runtime_conflicts;
use crate::modules::workspace::application::scanner::conflict::ConflictInfo;
use crate::modules::workspace::application::scanner::core::walker;
use crate::shared::errors::ScannerError;
use std::path::Path;

/// Detect conflicts by scanning the entire mods folder for INI files.
pub fn detect_conflicts_in_folder_service(
    mods_path: &Path,
) -> Result<Vec<ConflictInfo>, ScannerError> {
    // Use walker to find all mod folders
    let candidates = walker::scan_mod_folders(mods_path)?;

    let mod_roots = candidates
        .into_iter()
        .filter(|candidate| !candidate.is_disabled)
        .map(|candidate| candidate.path)
        .collect::<Vec<_>>();

    Ok(detect_runtime_conflicts(&mod_roots))
}
