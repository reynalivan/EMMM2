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

/// A keybinding associated with its source mod name for display in the overlay.
#[derive(Debug, Clone)]
pub struct SourceKeyBinding {
    pub mod_name: String,
    pub keybinds: Vec<KeyBinding>,
}

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
            parts.push(format!("Key: {}", trimmed));
        }
    }

    if let Some(ref back) = kb.back {
        let trimmed = back.trim();
        if !trimmed.is_empty() {
            parts.push(format!("Back: {}", trimmed));
        }
    }

    if parts.is_empty() {
        return format!("Toggle: {} (no key assigned)", kb.section_name);
    }

    parts.push(format!("Toggle: {}", kb.section_name));
    parts.join("\n")
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
    } else {
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
            } else {
                for kb in &source.keybinds {
                    lines.push(format_keybind_line(kb));
                }
            }
        }
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
/// Also writes `_fallback.txt` with a default message.
pub fn write_keybind_files(
    output_dir: &Path,
    matches: &[MatchResult],
    sources_per_object: &HashMap<String, Vec<SourceKeyBinding>>,
    safe_mode: bool,
    overlay_toggle_key: String,
) -> Result<Vec<PathBuf>, String> {
    let mut written_files = Vec::new();

    for match_result in matches {
        let sources = sources_per_object
            .get(&match_result.object_name)
            .cloned()
            .unwrap_or_default();

        let content =
            generate_keybind_text(&match_result.object_name, &sources, &overlay_toggle_key);

        // Write one file per sentinel hash
        for sentinel in &match_result.sentinel_hashes {
            let file_path = output_dir.join(format!("{sentinel}.txt"));
            atomic_write(&file_path, &content)?;
            written_files.push(file_path);
        }
    }

    // Write fallback file
    let fallback_content = if safe_mode {
        format!(
            "Safe Mode: ON\nNo character detected\n\n[{}] Toggle Overlay",
            overlay_toggle_key.to_uppercase()
        )
    } else {
        format!(
            "No character detected\n\n[{}] Toggle Overlay",
            overlay_toggle_key.to_uppercase()
        )
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
    pub conflict_count: Option<usize>,
}

/// Generate status banner text content.
pub fn generate_status_text(
    fields: &StatusFields,
    hotkey_config: &crate::services::hotkeys::HotkeyConfig,
) -> String {
    let safe_label = if fields.safe_mode { "On" } else { "Off" };
    let f5_key = hotkey_config.toggle_safe_mode.to_uppercase();
    let left_side = format!("Safe: {} [{}]", safe_label, f5_key);
    let right_side = if let Some(ref preset) = fields.preset_name {
        let shift_f6 = hotkey_config.prev_preset.to_uppercase();
        let f6 = hotkey_config.next_preset.to_uppercase();
        format!("Preset: {} [{}] [{}]", preset, shift_f6, f6)
    } else {
        "".to_string()
    };
    if right_side.is_empty() {
        left_side
    } else {
        format!("{}  |  {}", left_side, right_side)
    }
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
            let value = value_part.trim().trim_end_matches([';', '#']);
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
/// Uses the `help.ini` (3DMigoto FormatText) pipeline exclusively — the only
/// text renderer available without a custom shader. All text overlays are sent
/// via `pre Resource\ShaderFixes\help.ini\Notification` + `FormatText`.
///
/// Design principles (aligned with reference _KeyViewer):
/// - `$kv_has_active` reset per-frame via `post` in [Present]; set by TextureOverride on hit
/// - help.ini has ONE Notification slot — SINGLE decision tree: KeyViewer > Status
/// - `notification_timeout` set to ~infinity for persistence (no auto-hide)
/// - Stale detection at 1.5s (not 0.5s) to prevent camera-pan flicker
/// - `type = cycle` toggle key for sticky on/off
/// - All buffer resources: `type = buffer` + `format = R8_UINT` (lowercase, correct case)
pub fn generate_keyviewer_ini(
    matches: &[MatchResult],
    toggle_key: &str,
    keybinds_dir: &str,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // ── Header ───────────────────────────────────────────────────────────────
    lines.push("; =================================================================".to_string());
    lines.push(";   KeyViewer.ini — Auto-generated by EMMM".to_string());
    lines
        .push(";   Do NOT edit manually. Regenerated on mod/preset/safe-mode changes.".to_string());
    lines.push("; =================================================================".to_string());
    lines.push(String::new());

    // ── Global state variables ─────────────────────────────────────────────
    // $kv_active      = 0|1   master overlay toggle
    // $kv_has_active  = 0|1   set by TextureOverride each frame; reset by post in [Present]
    // $kv_active_code = 0xH   hash of the currently detected object
    // $kv_last_seen   = time  float timestamp of last successful hash detection
    lines.push("[Constants]".to_string());
    lines.push("global $kv_active = 0".to_string());
    lines.push("global $kv_has_active = 0".to_string());
    lines.push("global $kv_active_code = 0".to_string());
    lines.push("global $kv_last_seen = 0".to_string());
    lines.push(String::new());

    // ── Toggle key ─────────────────────────────────────────────────────────
    lines.push("; Press to show/hide the overlay".to_string());
    lines.push("[KeyEMMM_Toggle]".to_string());
    lines.push(format!("key = {toggle_key}"));
    lines.push("type = cycle".to_string());
    lines.push("$kv_active = 0, 1".to_string());
    lines.push(String::new());

    // ── TextureOverride sections (per sentinel hash) ────────────────────────
    // Fire every frame the hash is on screen. Sets $kv_has_active + $kv_active_code.
    // [Present] resets $kv_has_active via `post` before these run, so the value is
    // always fresh: 0 means nothing matched this frame.
    if !matches.is_empty() {
        lines.push(
            "; ─── Hash detection — fires when object is on screen ───────────────────".to_string(),
        );
        for match_result in matches {
            for (i, sentinel) in match_result.sentinel_hashes.iter().enumerate() {
                let section_name = format!(
                    "TextureOverride_EMM_{}_S{}",
                    match_result.object_name.replace([' ', '-'], "_"),
                    i
                );
                lines.push(format!("[{section_name}]"));
                lines.push(format!("hash = {sentinel}"));
                lines.push("$kv_has_active = 1".to_string());
                lines.push(format!("$kv_active_code = 0x{sentinel}"));
                lines.push("$kv_last_seen = time".to_string());
                lines.push(String::new());
            }
        }
    }

    // ── Present (per-frame driver) ─────────────────────────────────────────
    lines.push("[Present]".to_string());
    lines.push(
        "; Reset detection flag each frame. TextureOverride re-sets it if hash hits.".to_string(),
    );
    lines.push("post $kv_has_active = 0".to_string());
    lines.push(String::new());
    lines.push(
        "; Stale detection: clear active_code if no hash seen for > 1.5 seconds.".to_string(),
    );
    lines.push("; 1.5s threshold prevents flicker during camera panning.".to_string());
    lines.push("if time - $kv_last_seen > 1.5".to_string());
    lines.push("    $kv_active_code = 0".to_string());
    lines.push("endif".to_string());
    lines.push(String::new());
    lines.push("run = CommandList_EMM_Render".to_string());
    lines.push(String::new());

    // ── Render command list ─────────────────────────────────────────────────
    // SINGLE DECISION TREE. help.ini has exactly one Notification slot.
    // Writing it twice (last write wins) causes overlay conflicts.
    // Rule: if character is active → show KeyViewer; else → show Status banner.
    lines.push(
        "; ─── Render logic — help.ini pipeline ────────────────────────────────────".to_string(),
    );
    lines.push(
        "; help.ini supports ONE ResourceNotification slot — single decision tree.".to_string(),
    );
    lines.push("; Priority: per-character KeyViewer > global Status banner.".to_string());
    lines.push("[CommandList_EMM_Render]".to_string());
    lines.push("if $kv_active == 1".to_string());
    lines.push(String::new());
    lines.push("    if $kv_has_active == 1".to_string());
    lines.push(
        "        ; ── Character detected: show per-character keybind text ──────────".to_string(),
    );
    for match_result in matches {
        for sentinel in &match_result.sentinel_hashes {
            let resource_name = format!("ResourceKeyViewer_{sentinel}");
            lines.push(format!("        if $kv_active_code == 0x{sentinel}"));
            lines.push(format!(
                "            pre Resource\\ShaderFixes\\help.ini\\Notification = ref {resource_name}"
            ));
            lines.push("        endif".to_string());
        }
    }
    lines.push("    else".to_string());
    lines.push(
        "        ; ── No character detected: show status banner ──────────────────".to_string(),
    );
    lines.push(
        "        pre Resource\\ShaderFixes\\help.ini\\Notification = ref ResourceStatus"
            .to_string(),
    );
    lines.push("    endif".to_string());
    lines.push(String::new());
    lines.push(
        "    pre Resource\\ShaderFixes\\help.ini\\NotificationParams = ref ResourceBox".to_string(),
    );
    lines.push("    pre run = CustomShader\\ShaderFixes\\help.ini\\FormatText".to_string());
    lines.push("    $\\ShaderFixes\\help.ini\\notification_timeout = 1000000000.0".to_string());
    lines.push(String::new());
    lines.push("else".to_string());
    lines.push(
        "    ; Overlay toggled OFF — null the notification slot so nothing renders".to_string(),
    );
    lines.push("    Resource\\ShaderFixes\\help.ini\\Notification = null".to_string());
    lines.push("endif".to_string());
    lines.push(String::new());

    // ── ResourceBox — overlay layout params ───────────────────────────────
    // Matches reference ResourceArea format (single-line StructuredBuffer):
    //   pos(x y x2 y2)  textRGBA  bgRGBA  border(x y)  align(h v)  anchor  scale
    lines.push(
        "; ─── Layout/styling for the overlay box ──────────────────────────────────".to_string(),
    );
    lines.push("[ResourceBox]".to_string());
    lines.push("type = StructuredBuffer".to_string());
    lines.push("array = 1".to_string());
    lines.push("data = R32_FLOAT  -0.97 -1 1 1      0 0.55 0.75 1    0 0 0 0.99   0.01 0.01     1 2   0  1.0".to_string());
    lines.push(String::new());

    // ── ResourceStatus — status banner text ────────────────────────────────
    lines.push("; Status banner (shown when no character is active)".to_string());
    lines.push("[ResourceStatus]".to_string());
    lines.push("type = buffer".to_string());
    lines.push("format = R8_UINT".to_string());
    lines.push("filename = .emmm_data/status/runtime_status.txt".to_string());
    lines.push(String::new());

    // ── Per-sentinel keybind text resources ───────────────────────────────
    lines.push(
        "; ─── Per-character keybind text resources ────────────────────────────────".to_string(),
    );
    for match_result in matches {
        for sentinel in &match_result.sentinel_hashes {
            lines.push(format!("[ResourceKeyViewer_{sentinel}]"));
            lines.push("type = buffer".to_string());
            lines.push("format = R8_UINT".to_string());
            lines.push(format!("filename = {keybinds_dir}/{sentinel}.txt"));
            lines.push(String::new());
        }
    }

    // Fallback (no matched hash, but $kv_has_active was 1 — shouldn't happen but safety net)
    lines.push("[ResourceKeyViewer_Fallback]".to_string());
    lines.push("type = buffer".to_string());
    lines.push("format = R8_UINT".to_string());
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
) -> Result<(), String> {
    let content = generate_keyviewer_ini(matches, toggle_key, keybinds_dir);
    atomic_write(output_path, &content)
}
