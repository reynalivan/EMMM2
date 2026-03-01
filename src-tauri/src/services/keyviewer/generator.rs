//! File generation pipeline — keybind text, KeyViewer.ini, status banner, reload key discovery.
//!
//! Generates the files consumed by 3DMigoto at runtime:
//! - `EMM2/keybinds/active/<sentinel_hash>.txt` — per-object keybind text
//! - `EMM2/keybinds/active/_fallback.txt` — default text when no object matched
//! - `Mods/EMM2_System/KeyViewer.ini` — 3DMigoto runtime overlay
//! - `EMM2/status/runtime_status.txt` — in-game status banner
//!
//! All writes are atomic (`.tmp` → rename).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::services::ini::document::KeyBinding;
use crate::services::keyviewer::matcher::MatchResult;

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

// ─── Keybind Text Generation ─────────────────────────────────────────────────

/// Format a single keybind as a human-readable line.
///
/// Examples:
/// - `"[KeyToggleBody] Key: 1"` (key only)
/// - `"[KeyToggleBody] Key: 1 | Back: 2"` (key + back)
fn format_keybind_line(kb: &KeyBinding) -> String {
    let mut parts = Vec::new();

    if let Some(ref key) = kb.key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            parts.push(format!("Key: {trimmed}"));
        }
    }

    if let Some(ref back) = kb.back {
        let trimmed = back.trim();
        if !trimmed.is_empty() {
            parts.push(format!("Back: {trimmed}"));
        }
    }

    if parts.is_empty() {
        return format!("[{}] (no key assigned)", kb.section_name);
    }

    format!("[{}] {}", kb.section_name, parts.join(" | "))
}

/// Generate keybind text content for a single object.
///
/// Returns human-readable lines, one per keybind.
pub fn generate_keybind_text(object_name: &str, keybinds: &[KeyBinding]) -> String {
    if keybinds.is_empty() {
        return format!("{object_name}: No keybinds found");
    }

    let mut lines = vec![format!("=== {object_name} ===")];
    for kb in keybinds {
        lines.push(format_keybind_line(kb));
    }
    lines.join("\n")
}

/// Generate and write keybind text files for all matched objects.
///
/// For each match result, writes `<sentinel_hash>.txt` to the output directory.
/// Also writes `_fallback.txt` with a default message.
pub fn write_keybind_files(
    output_dir: &Path,
    matches: &[MatchResult],
    keybinds_per_object: &HashMap<String, Vec<KeyBinding>>,
    safe_mode: bool,
) -> Result<Vec<PathBuf>, String> {
    let mut written_files = Vec::new();

    for match_result in matches {
        let keybinds = keybinds_per_object
            .get(&match_result.object_name)
            .cloned()
            .unwrap_or_default();

        let content = generate_keybind_text(&match_result.object_name, &keybinds);

        // Write one file per sentinel hash
        for sentinel in &match_result.sentinel_hashes {
            let file_path = output_dir.join(format!("{sentinel}.txt"));
            atomic_write(&file_path, &content)?;
            written_files.push(file_path);
        }
    }

    // Write fallback file
    let fallback_content = if safe_mode {
        "Safe Mode: ON\nNo character detected".to_string()
    } else {
        "No character detected".to_string()
    };
    let fallback_path = output_dir.join("_fallback.txt");
    atomic_write(&fallback_path, &fallback_content)?;
    written_files.push(fallback_path);

    Ok(written_files)
}

// ─── Status Banner ───────────────────────────────────────────────────────────

/// Status banner fields for `runtime_status.txt`.
#[derive(Debug, Clone, Default)]
pub struct StatusFields {
    pub safe_mode: bool,
    pub preset_name: Option<String>,
    pub folder_name: Option<String>,
    pub scope_name: Option<String>,
}

/// Generate status banner text content.
///
/// Must be ≤ 10 lines, ≤ 4KB per req-42 spec.
pub fn generate_status_text(fields: &StatusFields) -> String {
    let mut lines = vec![
        "EMM2 Runtime".to_string(),
        format!("Safe: {}", if fields.safe_mode { "ON" } else { "OFF" }),
    ];

    if let Some(ref preset) = fields.preset_name {
        lines.push(format!("Preset: {preset}"));
    }
    if let Some(ref folder) = fields.folder_name {
        lines.push(format!("Folder: {folder}"));
    }
    if let Some(ref scope) = fields.scope_name {
        lines.push(format!("Scope: {scope}"));
    }

    lines.join("\n")
}

/// Write status banner atomically. Returns the path written.
pub fn write_status_file(status_dir: &Path, fields: &StatusFields) -> Result<PathBuf, String> {
    let content = generate_status_text(fields);

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

// ─── d3dx.ini Reload Key Discovery ──────────────────────────────────────────

/// The discovered reload key configuration from d3dx.ini.
#[derive(Debug, Clone)]
pub struct ReloadKeyConfig {
    /// The key that triggers `reload_fixes` (e.g. "F10", "F5").
    pub reload_fixes_key: String,
    /// Whether this was auto-discovered or is the fallback default.
    pub is_fallback: bool,
}

impl Default for ReloadKeyConfig {
    fn default() -> Self {
        Self {
            reload_fixes_key: "F10".to_string(),
            is_fallback: true,
        }
    }
}

/// Discover the reload key from a d3dx.ini file.
///
/// Scans `[Key*]` sections looking for `type = reload_fixes` or `type = reload_config`,
/// then reads the `key` assignment. Falls back to `F10` if not found.
pub fn discover_reload_key(d3dx_ini_path: &Path) -> ReloadKeyConfig {
    let content = match fs::read_to_string(d3dx_ini_path) {
        Ok(c) => c,
        Err(_) => return ReloadKeyConfig::default(),
    };

    let mut current_section: Option<String> = None;
    let mut in_key_section = false;
    let mut section_type: Option<String> = None;
    let mut section_key: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Section header
        if trimmed.starts_with('[') {
            // Before moving to next section, check if previous section had reload_fixes
            if in_key_section {
                if let (Some(ref t), Some(ref k)) = (&section_type, &section_key) {
                    if t.eq_ignore_ascii_case("reload_fixes") {
                        return ReloadKeyConfig {
                            reload_fixes_key: k.clone(),
                            is_fallback: false,
                        };
                    }
                }
            }

            if let Some(end) = trimmed.find(']') {
                let section_name = trimmed[1..end].trim();
                in_key_section = section_name.to_ascii_lowercase().starts_with("key");
                current_section = Some(section_name.to_string());
                section_type = None;
                section_key = None;
            }
            continue;
        }

        if !in_key_section || current_section.is_none() {
            continue;
        }

        // Parse key=value
        if let Some((key_part, value_part)) = trimmed.split_once('=') {
            let key_name = key_part.trim().to_ascii_lowercase();
            let value = value_part
                .trim()
                .trim_end_matches(|c: char| c == ';' || c == '#');
            let value = value.trim();

            match key_name.as_str() {
                "type" => section_type = Some(value.to_string()),
                "key" => section_key = Some(value.to_string()),
                _ => {}
            }
        }
    }

    // Check the last section
    if in_key_section {
        if let (Some(ref t), Some(ref k)) = (&section_type, &section_key) {
            if t.eq_ignore_ascii_case("reload_fixes") {
                return ReloadKeyConfig {
                    reload_fixes_key: k.clone(),
                    is_fallback: false,
                };
            }
        }
    }

    ReloadKeyConfig::default()
}

// ─── KeyViewer.ini Generation ────────────────────────────────────────────────

/// Generate KeyViewer.ini content for 3DMigoto runtime overlay.
///
/// This INI file contains:
/// - Global variables for overlay state management
/// - Toggle key section (show/hide overlay)
/// - Per-object TextureOverride sections using sentinel hashes
/// - Present section for TTL-based overlay display
pub fn generate_keyviewer_ini(
    matches: &[MatchResult],
    toggle_key: &str,
    keybinds_dir: &str,
    status_ttl: f32,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Header
    lines.push("; KeyViewer.ini — Auto-generated by EMMM2".to_string());
    lines.push(
        "; Do NOT edit manually. File is regenerated on mod/preset/safe-mode changes.".to_string(),
    );
    lines.push(String::new());

    // Global variables
    lines.push("[Constants]".to_string());
    lines.push("global $kv_visible = 0".to_string());
    lines.push(format!("global $kv_ttl = {status_ttl:.1}"));
    lines.push("global $kv_active_code = 0".to_string());
    lines.push("global $kv_detection_frame = 0".to_string());
    lines.push(String::new());

    // Toggle key section
    lines.push("[KeyToggleOverlay]".to_string());
    lines.push(format!("key = {toggle_key}"));
    lines.push("$kv_visible = 1 - $kv_visible".to_string());
    lines.push(String::new());

    // Per-object sentinel detection sections
    for match_result in matches {
        for (i, sentinel) in match_result.sentinel_hashes.iter().enumerate() {
            let section_name = format!(
                "TextureOverride_KV_{}_S{}",
                match_result.object_name.replace(' ', "_"),
                i
            );
            lines.push(format!("[{section_name}]"));
            lines.push(format!("hash = {sentinel}"));
            lines.push(format!("$kv_active_code = 0x{}", sentinel));
            lines.push("$kv_detection_frame = time".to_string());
            lines.push(String::new());
        }
    }

    // Present section — runs per-frame
    lines.push("[Present]".to_string());
    lines.push("; Reset detection if stale (object not on screen)".to_string());
    lines.push("if time - $kv_detection_frame > 0.5".to_string());
    lines.push("  $kv_active_code = 0".to_string());
    lines.push("endif".to_string());
    lines.push(String::new());

    // Resource sections for keybind text loading
    lines.push("; Keybind text resources — loaded by sentinel hash".to_string());
    for match_result in matches {
        for sentinel in &match_result.sentinel_hashes {
            let resource_name = format!("ResourceKeybindText_{}", sentinel);
            lines.push(format!("[{resource_name}]"));
            lines.push("type = Buffer".to_string());
            lines.push(format!("filename = {keybinds_dir}/{sentinel}.txt"));
            lines.push(String::new());
        }
    }

    // Fallback resource
    lines.push("[ResourceKeybindText_Fallback]".to_string());
    lines.push("type = Buffer".to_string());
    lines.push(format!("filename = {keybinds_dir}/_fallback.txt"));
    lines.push(String::new());

    lines.join("\n")
}

/// Generate and write KeyViewer.ini atomically.
pub fn write_keyviewer_ini(
    output_path: &Path,
    matches: &[MatchResult],
    toggle_key: &str,
    keybinds_dir: &str,
    status_ttl: f32,
) -> Result<(), String> {
    let content = generate_keyviewer_ini(matches, toggle_key, keybinds_dir, status_ttl);
    atomic_write(output_path, &content)
}
