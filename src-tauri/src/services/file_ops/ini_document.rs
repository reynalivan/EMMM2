//! Lossless INI read/discovery model for Epic 6.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\[([^\]]+)\]\s*$").expect("valid section regex"));
static VARIABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(\$[A-Za-z_][A-Za-z0-9_\.]*)\s*=\s*([^;#\r\n]+)")
        .expect("valid variable regex")
});
static KEY_BACK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(key|back)\s*=\s*([^;#\r\n]+)").expect("valid key regex"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IniReadMode {
    Structured,
    RawFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewlineStyle {
    Lf,
    CrLf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IniVariable {
    pub name: String,
    pub value: String,
    pub line_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyBinding {
    pub section_name: String,
    pub key: Option<String>,
    pub back: Option<String>,
    pub key_line_idx: Option<usize>,
    pub back_line_idx: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IniDocument {
    pub file_path: PathBuf,
    pub raw_lines: Vec<String>,
    pub variables: Vec<IniVariable>,
    pub key_bindings: Vec<KeyBinding>,
    pub had_bom: bool,
    pub newline_style: NewlineStyle,
    pub mode: IniReadMode,
}

pub fn list_ini_files(mod_path: &Path) -> Result<Vec<PathBuf>, String> {
    if !mod_path.exists() || !mod_path.is_dir() {
        return Err(format!("Invalid mod path: {}", mod_path.display()));
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(mod_path)
        .map_err(|e| format!("Failed to read mod folder: {e}"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("ini"))
                .unwrap_or(false)
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| !name.eq_ignore_ascii_case("desktop.ini"))
                .unwrap_or(true)
        })
        .collect();

    entries.sort_by(|a, b| {
        let an = a
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let bn = b
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        an.to_lowercase().cmp(&bn.to_lowercase())
    });

    Ok(entries)
}

pub fn read_ini_document(file_path: &Path) -> Result<IniDocument, String> {
    if !file_path.exists() {
        return Err(format!("INI file not found: {}", file_path.display()));
    }

    let bytes = fs::read(file_path).map_err(|e| format!("Failed to read INI file: {e}"))?;

    let had_bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
    let content_bytes = if had_bom { &bytes[3..] } else { &bytes[..] };

    let newline_style = if content_bytes.windows(2).any(|w| w == b"\r\n") {
        NewlineStyle::CrLf
    } else {
        NewlineStyle::Lf
    };

    let (text, utf8_ok) = match String::from_utf8(content_bytes.to_vec()) {
        Ok(s) => (s, true),
        Err(_) => (String::from_utf8_lossy(content_bytes).to_string(), false),
    };

    let raw_lines: Vec<String> = text.lines().map(ToString::to_string).collect();

    if !utf8_ok {
        return Ok(IniDocument {
            file_path: file_path.to_path_buf(),
            raw_lines,
            variables: Vec::new(),
            key_bindings: Vec::new(),
            had_bom,
            newline_style,
            mode: IniReadMode::RawFallback,
        });
    }

    let mut variables: Vec<IniVariable> = Vec::new();
    let mut key_bindings: HashMap<String, KeyBinding> = HashMap::new();
    let mut current_section: Option<String> = None;
    let mut malformed_section = false;

    for (idx, line) in raw_lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && !trimmed.contains(']') {
            malformed_section = true;
            break;
        }

        if let Some(caps) = SECTION_RE.captures(trimmed) {
            current_section = Some(caps[1].trim().to_string());
            continue;
        }

        if let Some(caps) = VARIABLE_RE.captures(line) {
            variables.push(IniVariable {
                name: caps[1].trim().to_string(),
                value: caps[2].trim().to_string(),
                line_idx: idx,
            });
            continue;
        }

        let Some(section_name) = current_section.as_ref() else {
            continue;
        };

        if !section_name.to_ascii_lowercase().starts_with("key") {
            continue;
        }

        if let Some(caps) = KEY_BACK_RE.captures(line) {
            let field = caps[1].to_ascii_lowercase();
            let value = caps[2].trim().to_string();
            let entry = key_bindings
                .entry(section_name.clone())
                .or_insert_with(|| KeyBinding {
                    section_name: section_name.clone(),
                    key: None,
                    back: None,
                    key_line_idx: None,
                    back_line_idx: None,
                });

            if field == "key" {
                entry.key = Some(value);
                entry.key_line_idx = Some(idx);
            } else {
                entry.back = Some(value);
                entry.back_line_idx = Some(idx);
            }
        }
    }

    if malformed_section {
        return Ok(IniDocument {
            file_path: file_path.to_path_buf(),
            raw_lines,
            variables: Vec::new(),
            key_bindings: Vec::new(),
            had_bom,
            newline_style,
            mode: IniReadMode::RawFallback,
        });
    }

    let mut key_bindings: Vec<KeyBinding> = key_bindings.into_values().collect();
    key_bindings.sort_by(|a, b| a.section_name.cmp(&b.section_name));

    Ok(IniDocument {
        file_path: file_path.to_path_buf(),
        raw_lines,
        variables,
        key_bindings,
        had_bom,
        newline_style,
        mode: IniReadMode::Structured,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Covers: NC-6.3-02 (Missing INI File)
    #[test]
    fn test_list_ini_files_filters_non_ini_noise() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("ModA");
        fs::create_dir(&mod_dir).unwrap();

        fs::write(mod_dir.join("desktop.ini"), "[.ShellClassInfo]").unwrap();
        fs::write(mod_dir.join("d3dx.ini"), "[Constants]\n$swapvar = 1").unwrap();
        fs::write(mod_dir.join("config.INI"), "[KeySwap]\nkey = v").unwrap();
        fs::write(mod_dir.join("readme.txt"), "notes").unwrap();

        let files = list_ini_files(&mod_dir).unwrap();
        let names: Vec<String> = files
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["config.INI", "d3dx.ini"]);
    }

    // Covers: TC-6.3-01 (Parse Variables)
    #[test]
    fn test_read_ini_document_parses_variable_lines() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("config.ini");
        fs::write(&ini_path, "[Constants]\n$swapvar = 1\n").unwrap();

        let doc = read_ini_document(&ini_path).unwrap();

        assert_eq!(doc.mode, IniReadMode::Structured);
        assert_eq!(doc.variables.len(), 1);
        assert_eq!(doc.variables[0].name, "$swapvar");
        assert_eq!(doc.variables[0].value, "1");
    }

    // Covers: TC-6.3-01 (Keybindings)
    #[test]
    fn test_read_ini_document_parses_keybinding_section() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("config.ini");
        fs::write(&ini_path, "[KeyChangeDress]\nkey = v\nback = b\n").unwrap();

        let doc = read_ini_document(&ini_path).unwrap();

        assert_eq!(doc.key_bindings.len(), 1);
        assert_eq!(doc.key_bindings[0].section_name, "KeyChangeDress");
        assert_eq!(doc.key_bindings[0].key.as_deref(), Some("v"));
        assert_eq!(doc.key_bindings[0].back.as_deref(), Some("b"));
    }

    // Covers: TC-6.3-03 (BOM Handling)
    #[test]
    fn test_read_ini_document_detects_and_strips_bom_in_memory() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("bom.ini");

        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"[Constants]\n$var = 1\n");
        fs::write(&ini_path, bytes).unwrap();

        let doc = read_ini_document(&ini_path).unwrap();

        assert!(doc.had_bom);
        assert!(!doc.raw_lines[0].starts_with('\u{FEFF}'));
        assert_eq!(doc.mode, IniReadMode::Structured);
    }

    // Covers: NC-6.3-01 (Malformed Syntax)
    #[test]
    fn test_read_ini_document_malformed_section_falls_back_to_raw_mode() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("broken.ini");
        fs::write(&ini_path, "[Section\n$var = 1\n").unwrap();

        let doc = read_ini_document(&ini_path).unwrap();
        assert_eq!(doc.mode, IniReadMode::RawFallback);
    }

    // Covers: EC-6.01 (Shift-JIS / GBK INI)
    #[test]
    fn test_read_ini_document_non_utf8_falls_back_to_raw_mode() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("encoded.ini");
        fs::write(&ini_path, [0x82_u8, 0xA0_u8, 0x82_u8]).unwrap();

        let doc = read_ini_document(&ini_path).unwrap();
        assert_eq!(doc.mode, IniReadMode::RawFallback);
    }

    // Covers: DI-6.02 (BOM Preservation metadata), EC-6.05
    #[test]
    fn test_read_ini_document_detects_crlf_newline_style() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("newline.ini");
        fs::write(&ini_path, "[Constants]\r\n$var = 1\r\n").unwrap();

        let doc = read_ini_document(&ini_path).unwrap();
        assert_eq!(doc.newline_style, NewlineStyle::CrLf);
    }
}
