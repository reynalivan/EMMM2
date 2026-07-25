use std::path::{Path, PathBuf};

use super::atomic::atomic_write;

// ─── Status Banner ───────────────────────────────────────────────────────────

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
        let shift_f6 = hotkey_config.prev_preset.to_uppercase();
        let f6 = hotkey_config.next_preset.to_uppercase();
        segments.push(format!("Preset: {} [{}] [{}]", preset, shift_f6, f6));
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
) -> Result<PathBuf, String> {
    let content = generate_status_text(fields, hotkey_config);

    // Enforce req-42 constraints
    let line_count = content.lines().count();
    if line_count > 10 {
        return Err(format!("Status banner exceeds 10 lines (got {line_count})"));
    }
    if content.len() > 4096 {
        return Err(format!(
            "Status banner exceeds 4KB (got {} bytes)",
            content.len()
        ));
    }

    let path = status_dir.join("runtime_status.txt");
    atomic_write(&path, &content)?;
    Ok(path)
}

/// Clear the status banner (delete or empty the file).
pub fn clear_status_file(status_dir: &Path) -> Result<(), String> {
    let path = status_dir.join("runtime_status.txt");
    if path.exists() {
        atomic_write(&path, "")?;
    }
    Ok(())
}
