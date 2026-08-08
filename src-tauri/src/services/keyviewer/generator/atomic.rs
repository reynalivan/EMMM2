use crate::domain::errors::AppError;
use std::fs;
use std::path::Path;

// ─── Atomic Write ────────────────────────────────────────────────────────────

/// Write a file atomically: write to `.tmp`, then rename to final path.
/// Ensures readers always see a complete file.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    let tmp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&tmp_path, content)?;

    Ok(fs::rename(&tmp_path, path)?)
}
