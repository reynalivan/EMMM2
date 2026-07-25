use std::fs;
use std::path::Path;

// ─── Atomic Write ────────────────────────────────────────────────────────────

/// Write a file atomically: write to `.tmp`, then rename to final path.
/// Ensures readers always see a complete file.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }

    fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write {}: {e}", tmp_path.display()))?;

    fs::rename(&tmp_path, path).map_err(|e| {
        format!(
            "Failed to rename {} → {}: {e}",
            tmp_path.display(),
            path.display()
        )
    })
}
