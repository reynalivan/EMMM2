use std::fs;
use std::path::Path;

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
