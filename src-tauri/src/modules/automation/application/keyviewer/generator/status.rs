use std::path::{Path, PathBuf};

use crate::shared::errors::AppError;

use super::atomic::atomic_write;

const MAX_STATUS_BYTES: usize = 4096;
const MAX_STATUS_LINES: usize = 10;
const MAX_STATUS_VALUE_CHARS: usize = 160;

/// Runtime fields shown by the unified overlay status banner.
#[derive(Debug, Clone, Default)]
pub struct StatusFields {
    pub safe_mode: bool,
    pub preset_name: Option<String>,
    pub keyviewer_enabled: bool,
    pub reload_binding: Option<String>,
}

fn bounded_value(value: &str) -> String {
    let mut bounded = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(MAX_STATUS_VALUE_CHARS)
        .collect::<String>();
    if value.chars().count() > MAX_STATUS_VALUE_CHARS {
        bounded.push('…');
    }
    bounded
}

fn helper(binding: &str, enabled: bool) -> String {
    let binding = bounded_value(&binding.to_uppercase());
    if enabled {
        binding
    } else {
        format!("inactive ({binding})")
    }
}

/// Generate the persistent status banner. The binding text comes from the
/// same settings snapshot used to generate the 3DMigoto F7 binding.
pub fn generate_status_text(
    fields: &StatusFields,
    hotkey_config: &crate::modules::automation::application::hotkeys::HotkeyConfig,
) -> String {
    let safe = if fields.safe_mode { "ON" } else { "OFF" };
    let preset = bounded_value(fields.preset_name.as_deref().unwrap_or("None"));
    let enabled = hotkey_config.enabled;
    let keyviewer_enabled = fields.keyviewer_enabled;
    let reload = fields
        .reload_binding
        .as_deref()
        .map(|binding| bounded_value(&binding.to_uppercase()))
        .unwrap_or_else(|| "manual reload required".to_string());
    let text = [
        format!(
            "Safe Mode: {safe} | Toggle: {}",
            helper(&hotkey_config.safe_mode, enabled)
        ),
        format!("Collection: {preset}"),
        format!(
            "Previous: {} | Next: {}",
            helper(&hotkey_config.prev_preset, enabled),
            helper(&hotkey_config.next_preset, enabled)
        ),
        format!(
            "KeyViewer: {} | Toggle: {}",
            if keyviewer_enabled { "ON" } else { "inactive" },
            helper(&hotkey_config.toggle_overlay, keyviewer_enabled)
        ),
        format!("Reload config: {reload}"),
    ]
    .join("\n");
    debug_assert!(text.lines().count() <= MAX_STATUS_LINES);
    debug_assert!(text.len() <= MAX_STATUS_BYTES);
    text
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
