use std::path::{Path, PathBuf};

use super::atomic::atomic_write;
use crate::domain::errors::AppError;

// ─── Status Banner ───────────────────────────────────────────────────────────

/// Limits of the 3DMigoto text resource the overlay renders (req-42).
const MAX_STATUS_LINES: usize = 10;
const MAX_STATUS_BYTES: usize = 4096;

/// Status banner fields for `runtime_status.txt`.
#[derive(Debug, Clone, Default)]
pub struct StatusFields {
    pub safe_mode: bool,
    pub preset_name: Option<String>,
    pub folder_name: Option<String>,
    pub scope_name: Option<String>,
    pub conflict_count: Option<usize>,
}

/// Generate status banner text content.
pub fn generate_status_text(
    fields: &StatusFields,
    hotkey_config: &crate::services::hotkeys::HotkeyConfig,
) -> String {
    let mut segments = Vec::new();

    if let Some(ref preset) = fields.preset_name {
        // Named for the action, not the default key — these are rebindable.
        let prev_key = hotkey_config.prev_preset.to_uppercase();
        let next_key = hotkey_config.next_preset.to_uppercase();
        segments.push(format!("Preset: {} [{}] [{}]", preset, prev_key, next_key));
    }

    if let Some(ref folder) = fields.folder_name {
        segments.push(format!("Folder: {}", folder));
    }

    if let Some(ref scope) = fields.scope_name {
        segments.push(format!("Scope: {}", scope));
    }

    if let Some(conflict_count) = fields.conflict_count.filter(|count| *count > 0) {
        segments.push(format!("Conflicts: {}", conflict_count));
    }

    if segments.is_empty() {
        segments.push("Runtime ready".to_string());
    }

    segments.join("  |  ")
}

/// Write status banner atomically. Returns the path written.
pub fn write_status_file(
    status_dir: &Path,
    fields: &StatusFields,
    hotkey_config: &crate::services::hotkeys::HotkeyConfig,
) -> Result<PathBuf, AppError> {
    let content = generate_status_text(fields, hotkey_config);

    let line_count = content.lines().count();
    if line_count > MAX_STATUS_LINES {
        return Err(AppError::Internal(format!(
            "Status banner exceeds {MAX_STATUS_LINES} lines (got {line_count})"
        )));
    }
    if content.len() > MAX_STATUS_BYTES {
        return Err(AppError::Internal(format!(
            "Status banner exceeds {MAX_STATUS_BYTES} bytes (got {} bytes)",
            content.len()
        )));
    }

    let path = status_dir.join("runtime_status.txt");
    atomic_write(&path, &content)?;
    Ok(path)
}
