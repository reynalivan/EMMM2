use std::path::Path;
use anyhow::{Context, Result};

/// Safely renames a file, ensuring source exists and destination does not.
pub fn safe_rename(from: &Path, to: &Path) -> Result<()> {
    // 1. Pre-flight Checks
    if !from.exists() {
        return Err(anyhow::anyhow!("Source file not found: {:?}", from));
    }
    if to.exists() {
        return Err(anyhow::anyhow!("Destination already exists: {:?}", to));
    }

    // 2. Atomic Rename
    // Note: std::fs::rename IS atomic on POSIX, but mostly atomic on Windows (unless cross-drive)
    std::fs::rename(from, to)
        .with_context(|| format!("Failed to move {:?} to {:?}", from, to))?;
    
    Ok(())
}

/// Moves a file/folder to the system trash instead of permanent deletion.
pub fn safe_trash(target: &Path) -> Result<()> {
    if !target.exists() {
        return Ok(()); // Already gone, treat as success
    }

    trash::delete(target)
        .with_context(|| format!("Failed to move to trash: {:?}", target))?;
    
    Ok(())
}
