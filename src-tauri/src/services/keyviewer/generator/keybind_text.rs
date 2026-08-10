use crate::domain::errors::AppError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::services::ini::document::KeyBinding;
use crate::services::keyviewer::matcher::MatchResult;

use super::atomic::atomic_write;

/// A keybinding associated with its source mod name for display in the overlay.
#[derive(Debug, Clone)]
pub struct SourceKeyBinding {
    pub mod_name: String,
    pub keybinds: Vec<KeyBinding>,
}

// ─── Keybind Text Generation ─────────────────────────────────────────────────

/// Format a single keybind as a human-readable line.
///
/// Examples:
/// - `"[KeyToggleBody] Key: 1"` (key only)
/// - `"[KeyToggleBody] Key: 1 | Back: 2"` (key + back)
fn format_keybind_line(kb: &KeyBinding) -> String {
    let parts: Vec<String> = [("Key", &kb.key), ("Back", &kb.back)]
        .into_iter()
        .filter_map(|(label, raw)| {
            let value = raw.as_deref().map(str::trim).filter(|v| !v.is_empty())?;
            Some(format!("{label}: {value}"))
        })
        .collect();

    if parts.is_empty() {
        return format!("[{}] No key assigned", kb.section_name);
    }

    format!("[{}] {}", kb.section_name, parts.join(" | "))
}

/// Generate keybind text content for a single object, possibly from multiple mods.
pub fn generate_keybind_text(
    object_name: &str,
    sources: &[SourceKeyBinding],
    toggle_overlay_key: &str,
) -> String {
    let mut lines = vec![
        object_name.to_string(),
        "-".repeat(object_name.len().max(10)),
    ];

    if sources.is_empty() {
        lines.push("No keybinds found".to_string());
    }

    for (i, source) in sources.iter().enumerate() {
        if i > 0 {
            lines.push(String::new());
        }

        // Only show mod header if there are multiple mods or for clarity
        if sources.len() > 1 {
            lines.push(format!("[Mod: {}]", source.mod_name));
        }

        if source.keybinds.is_empty() {
            lines.push("  (No keybinds in this mod)".to_string());
            continue;
        }

        lines.extend(source.keybinds.iter().map(format_keybind_line));
    }

    lines.push(String::new());
    lines.push(format!(
        "[{}] Toggle Overlay",
        toggle_overlay_key.to_uppercase()
    ));
    lines.join("\n")
}

/// Generate and write keybind text files for all matched objects.
///
/// For each match result, writes `<sentinel_hash>.txt` to the output directory.
pub fn write_keybind_files(
    output_dir: &Path,
    matches: &[MatchResult],
    sources_per_object: &HashMap<String, Vec<SourceKeyBinding>>,
    overlay_toggle_key: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let mut written_files = Vec::new();

    for match_result in matches {
        let sources = sources_per_object
            .get(&match_result.object_name)
            .map(Vec::as_slice)
            .unwrap_or_default();

        let content = generate_keybind_text(&match_result.object_name, sources, overlay_toggle_key);

        // Write one file per sentinel hash
        for sentinel in &match_result.sentinel_hashes {
            let file_path = output_dir.join(format!("{sentinel}.txt"));
            atomic_write(&file_path, &content)?;
            written_files.push(file_path);
        }
    }

    Ok(written_files)
}
