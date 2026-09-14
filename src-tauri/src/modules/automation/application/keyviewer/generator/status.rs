use std::path::{Path, PathBuf};

use crate::shared::errors::AppError;

use super::atomic::atomic_write;

const MAX_STATUS_BYTES: usize = 4096;

/// Runtime fields shown by the unified overlay status banner.
#[derive(Debug, Clone, Default)]
pub struct StatusFields {
    pub safe_mode: bool,
    pub preset_name: Option<String>,
}

/// Generate the persistent status banner. The binding text comes from the
/// same settings snapshot used to generate the 3DMigoto F7 binding.
pub fn generate_status_text(
    fields: &StatusFields,
    hotkey_config: &crate::modules::automation::application::hotkeys::HotkeyConfig,
) -> String {
    let safe = if fields.safe_mode { "On" } else { "Off" };
    let preset = fields.preset_name.as_deref().unwrap_or("None");
    format!(
        "Safe: {safe} [{}] | Preset: {preset} [{}] [{}]",
        hotkey_config.safe_mode.to_uppercase(),
        hotkey_config.prev_preset.to_uppercase(),
        hotkey_config.next_preset.to_uppercase(),
    )
}

/// Write status banner atomically. Returns the path written.
pub fn write_status_file(
    status_dir: &Path,
    fields: &StatusFields,
    hotkey_config: &crate::modules::automation::application::hotkeys::HotkeyConfig,
) -> Result<PathBuf, AppError> {
    let content = generate_status_text(fields, hotkey_config);
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
