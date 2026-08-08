use std::fs;
use std::path::Path;

use crate::services::ini::document::decode_ini_bytes;

// ─── d3dx.ini Reload Key Discovery ──────────────────────────────────────────

/// Key 3DMigoto binds `reload_fixes` to out of the box.
pub const DEFAULT_RELOAD_KEY: &str = "F10";

/// The `type =` value that marks the section we are looking for.
const RELOAD_FIXES_TYPE: &str = "reload_fixes";

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
            reload_fixes_key: DEFAULT_RELOAD_KEY.to_string(),
            is_fallback: true,
        }
    }
}

/// The key of a finished `[Key*]` section, if it was the reload_fixes binding.
fn reload_key_of_section(section_type: &Option<String>, key: &Option<String>) -> Option<String> {
    let section_type = section_type.as_ref()?;
    section_type
        .eq_ignore_ascii_case(RELOAD_FIXES_TYPE)
        .then(|| key.clone())
        .flatten()
}

/// Discover the reload key from a d3dx.ini file.
///
/// Scans `[Key*]` sections looking for `type = reload_fixes`, then reads the
/// `key` assignment. Falls back to [`DEFAULT_RELOAD_KEY`] if not found.
pub fn discover_reload_key(d3dx_ini_path: &Path) -> ReloadKeyConfig {
    let Ok(bytes) = fs::read(d3dx_ini_path) else {
        return ReloadKeyConfig::default();
    };
    // Shared decoder: a BOM'd or Shift-JIS d3dx.ini must not silently fall back.
    let (content, _had_bom, _clean) = decode_ini_bytes(&bytes);

    let mut in_key_section = false;
    let mut section_type: Option<String> = None;
    let mut section_key: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            // A section ends where the next one begins; settle it before moving on.
            if in_key_section {
                if let Some(key) = reload_key_of_section(&section_type, &section_key) {
                    return ReloadKeyConfig {
                        reload_fixes_key: key,
                        is_fallback: false,
                    };
                }
            }

            if let Some(end) = trimmed.find(']') {
                let section_name = trimmed[1..end].trim();
                in_key_section = section_name
                    .get(..3)
                    .is_some_and(|head| head.eq_ignore_ascii_case("key"));
                section_type = None;
                section_key = None;
            }
            continue;
        }

        if !in_key_section {
            continue;
        }

        let Some((key_part, value_part)) = trimmed.split_once('=') else {
            continue;
        };
        let key_name = key_part.trim();
        let value = value_part.trim().trim_end_matches([';', '#']).trim();

        if key_name.eq_ignore_ascii_case("type") {
            section_type = Some(value.to_string());
        } else if key_name.eq_ignore_ascii_case("key") {
            section_key = Some(value.to_string());
        }
    }

    // The final section has no following header to close it.
    match reload_key_of_section(&section_type, &section_key).filter(|_| in_key_section) {
        Some(key) => ReloadKeyConfig {
            reload_fixes_key: key,
            is_fallback: false,
        },
        None => ReloadKeyConfig::default(),
    }
}
