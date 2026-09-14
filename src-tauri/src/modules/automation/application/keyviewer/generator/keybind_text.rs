use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::modules::automation::application::keyviewer::matcher::MatchResult;
use crate::modules::library::application::ini::document::KeyBinding;
use crate::shared::errors::AppError;

use super::atomic::atomic_write;

const MAX_KEYBIND_BYTES: usize = 8 * 1024;
const MAX_KEYBIND_LINES: usize = 60;
const TRUNCATION_MARKER: &str = "... (truncated; see the EMMM preview for the full list)";
const MAX_HEADING_BYTES: usize = 512;
const MAX_FOOTER_BYTES: usize = 256;

/// A keybinding associated with its source mod name for display in the overlay.
#[derive(Debug, Clone)]
pub struct SourceKeyBinding {
    pub mod_name: String,
    pub keybinds: Vec<KeyBinding>,
}

/// The deterministic filename for one matching character's generated text.
pub fn keybind_file_name(index: usize) -> String {
    format!("character_{index:03}.txt")
}

fn keybind_lines(keybind: &KeyBinding, show_section: bool) -> Vec<String> {
    let mut lines = Vec::new();
    if show_section {
        lines.push(format!("[{}]", keybind.section_name));
    }

    if let Some(key) = keybind
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        if keybind
            .binding_type
            .as_deref()
            .is_some_and(|binding_type| binding_type.eq_ignore_ascii_case("toggle"))
        {
            lines.push(format!("Toggle: {key}"));
        } else {
            lines.push(format!("Key: {key}"));
        }
    }
    if let Some(back) = keybind
        .back
        .as_deref()
        .map(str::trim)
        .filter(|back| !back.is_empty())
    {
        lines.push(format!("Back: {back}"));
    }

    lines
}

fn truncate_utf8(value: &str, maximum_bytes: usize) -> String {
    if value.len() <= maximum_bytes {
        return value.to_string();
    }
    let suffix = "...";
    let available = maximum_bytes.saturating_sub(suffix.len());
    let mut end = 0;
    for character in value.chars() {
        let next_end = end + character.len_utf8();
        if next_end > available {
            break;
        }
        end = next_end;
    }
    format!("{}{suffix}", &value[..end])
}

fn append_with_limits(
    lines: &mut Vec<String>,
    candidates: impl IntoIterator<Item = String>,
    footer: &str,
) {
    let mut truncated = false;

    for candidate in candidates {
        let existing_bytes = lines.iter().map(|line| line.len() + 1).sum::<usize>();
        let reserved_bytes = footer.len() + 2 + TRUNCATION_MARKER.len() + 1;
        let reserved_lines = 3;

        if lines.len() + reserved_lines >= MAX_KEYBIND_LINES
            || existing_bytes + candidate.len() + 1 + reserved_bytes > MAX_KEYBIND_BYTES
        {
            truncated = true;
            break;
        }

        lines.push(candidate);
    }

    if truncated {
        lines.push(TRUNCATION_MARKER.to_string());
    }
}

/// Generate keybind text for one character. Keys are copied directly from the
/// parsed INI; no binding type or modifier is inferred or rewritten.
pub fn generate_keybind_text(
    object_name: &str,
    sources: &[SourceKeyBinding],
    toggle_overlay_key: &str,
) -> String {
    let heading = truncate_utf8(object_name, MAX_HEADING_BYTES);
    let mut lines = vec![
        heading.clone(),
        "-".repeat(heading.chars().count().clamp(10, MAX_HEADING_BYTES)),
    ];
    let footer = truncate_utf8(
        &format!("[{}] Toggle Overlay", toggle_overlay_key.to_uppercase()),
        MAX_FOOTER_BYTES,
    );

    let source_count = sources.len();
    let candidates = sources.iter().flat_map(|source| {
        let mut source_lines = Vec::new();
        if source_count > 1 {
            source_lines.push(format!("[Mod: {}]", source.mod_name));
        }

        let show_section = source.keybinds.len() > 1;
        for keybind in &source.keybinds {
            source_lines.extend(keybind_lines(keybind, show_section));
        }

        source_lines
    });

    append_with_limits(&mut lines, candidates, &footer);
    lines.push(String::new());
    lines.push(footer);
    lines.join("\n")
}

/// Generate and write one deterministic text file for every matched object.
pub fn write_keybind_files(
    output_dir: &Path,
    matches: &[MatchResult],
    sources_per_object: &HashMap<String, Vec<SourceKeyBinding>>,
    overlay_toggle_key: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let mut written_files = Vec::new();

    for (index, match_result) in matches.iter().enumerate() {
        let sources = sources_per_object
            .get(&match_result.object_name)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let content = generate_keybind_text(&match_result.object_name, sources, overlay_toggle_key);
        let file_path = output_dir.join(keybind_file_name(index));
        atomic_write(&file_path, &content)?;
        written_files.push(file_path);
    }

    Ok(written_files)
}
