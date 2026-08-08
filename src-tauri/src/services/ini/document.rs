//! Lossless INI read/discovery model for Epic 6.

use crate::domain::errors::AppError;
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

/// Refuse to build an editable model for files this large — the editor holds
/// the whole document in memory and round-trips it on save.
const MAX_INI_BYTES: u64 = 2 * 1024 * 1024;

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// 3DMigoto keybind sections are named `[Key…]`.
const KEY_SECTION_PREFIX: &str = "key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum IniReadMode {
    Structured,
    RawFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum NewlineStyle {
    Lf,
    CrLf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct IniVariable {
    pub name: String,
    pub value: String,
    #[specta(type = f64)]
    pub line_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct KeyBinding {
    pub section_name: String,
    pub key: Option<String>,
    pub back: Option<String>,
    #[specta(type = f64)]
    pub key_line_idx: Option<usize>,
    #[specta(type = f64)]
    pub back_line_idx: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct IniDocument {
    pub file_path: std::path::PathBuf,
    pub raw_lines: Vec<String>,
    pub variables: Vec<IniVariable>,
    pub key_bindings: Vec<KeyBinding>,
    pub had_bom: bool,
    pub newline_style: NewlineStyle,
    pub mode: IniReadMode,
}

pub fn list_ini_files(mod_path: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !mod_path.exists() || !mod_path.is_dir() {
        return Err(AppError::Internal(format!(
            "Invalid mod path: {}",
            mod_path.display()
        )));
    }

    // Extension first, then file type from the directory entry: `path.is_file()`
    // would cost a fresh stat for every `.dds`/`.buf` in the folder too.
    let mut entries: Vec<PathBuf> = fs::read_dir(mod_path)?
        .flatten()
        .filter(|entry| {
            let path = entry.path();
            let is_ini = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"));
            let is_desktop_ini = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("desktop.ini"));

            is_ini
                && !is_desktop_ini
                && entry.file_type().is_ok_and(|file_type| file_type.is_file())
        })
        .map(|entry| entry.path())
        .collect();

    entries.sort_by_cached_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });

    Ok(entries)
}

/// Strip a UTF-8 BOM and decode INI bytes: UTF-8 first, Shift-JIS next (JP mod
/// tooling still writes it), lossy UTF-8 as the last resort. Returns
/// `(text, had_bom, clean)`; `clean == false` means the lossy fallback ran and
/// structured parsing should not trust the text.
pub fn decode_ini_bytes(bytes: &[u8]) -> (String, bool, bool) {
    let had_bom = bytes.starts_with(&UTF8_BOM);
    let content_bytes = if had_bom {
        &bytes[UTF8_BOM.len()..]
    } else {
        bytes
    };

    match String::from_utf8(content_bytes.to_vec()) {
        Ok(text) => (text, had_bom, true),
        Err(_) => {
            let (cow, _encoding, had_errors) = encoding_rs::SHIFT_JIS.decode(content_bytes);
            if !had_errors {
                (cow.into_owned(), had_bom, true)
            } else {
                (
                    String::from_utf8_lossy(content_bytes).to_string(),
                    had_bom,
                    false,
                )
            }
        }
    }
}

/// Structured view of an INI body, or `None` when the file is not safely
/// parseable and the caller must fall back to raw lines.
fn parse_structured(raw_lines: &[String]) -> Option<(Vec<IniVariable>, Vec<KeyBinding>)> {
    let mut variables: Vec<IniVariable> = Vec::new();
    let mut key_bindings: HashMap<String, KeyBinding> = HashMap::new();
    let mut key_section: Option<&str> = None;

    for (idx, line) in raw_lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            // A section header that never closes means the file is malformed.
            if !trimmed.contains(']') {
                return None;
            }
            if let Some(caps) = SECTION_RE.captures(trimmed) {
                // Whether this is a keybind section is fixed for the whole
                // section, so classify once here rather than per line.
                let name = caps.get(1).map(|m| m.as_str().trim()).unwrap_or_default();
                // `get` rather than slicing: a non-ASCII section name must not
                // panic on a byte index that lands mid-character.
                key_section = name
                    .get(..KEY_SECTION_PREFIX.len())
                    .filter(|head| head.eq_ignore_ascii_case(KEY_SECTION_PREFIX))
                    .map(|_| name);
                continue;
            }
        }

        if line.trim_start().starts_with('$') {
            if let Some(caps) = VARIABLE_RE.captures(line) {
                variables.push(IniVariable {
                    name: caps[1].trim().to_string(),
                    value: caps[2].trim().to_string(),
                    line_idx: idx,
                });
                continue;
            }
        }

        let Some(section_name) = key_section else {
            continue;
        };

        let Some(caps) = KEY_BACK_RE.captures(line) else {
            continue;
        };
        let entry = key_bindings
            .entry(section_name.to_string())
            .or_insert_with(|| KeyBinding {
                section_name: section_name.to_string(),
                key: None,
                back: None,
                key_line_idx: None,
                back_line_idx: None,
            });

        let value = caps[2].trim().to_string();
        if caps[1].eq_ignore_ascii_case("key") {
            entry.key = Some(value);
            entry.key_line_idx = Some(idx);
        } else {
            entry.back = Some(value);
            entry.back_line_idx = Some(idx);
        }
    }

    let mut key_bindings: Vec<KeyBinding> = key_bindings.into_values().collect();
    key_bindings.sort_by(|a, b| a.section_name.cmp(&b.section_name));

    Some((variables, key_bindings))
}

/// Largest INI the structured parser will accept, in bytes.
pub const MAX_PARSEABLE_INI_BYTES: u64 = MAX_INI_BYTES;

pub fn read_ini_document(file_path: &Path) -> Result<IniDocument, AppError> {
    let meta = fs::metadata(file_path)?;
    if meta.len() > MAX_INI_BYTES {
        return Err(AppError::Validation(
            "INI file is too large to edit safely (>2MB). Please use an external editor."
                .to_string(),
        ));
    }

    let bytes = fs::read(file_path)?;
    Ok(parse_ini_document(file_path, &bytes))
}

/// Parses bytes already in hand.
///
/// Split out of `read_ini_document` so the keyviewer harvest, which needs both
/// the raw text and the parsed keybinds of every INI, can read each file once
/// instead of once per consumer.
pub fn parse_ini_document(file_path: &Path, bytes: &[u8]) -> IniDocument {
    let newline_style = if bytes.windows(2).any(|w| w == b"\r\n") {
        NewlineStyle::CrLf
    } else {
        NewlineStyle::Lf
    };

    let (text, had_bom, utf8_ok) = decode_ini_bytes(bytes);

    let raw_lines: Vec<String> = text.lines().map(ToString::to_string).collect();

    // A lossy decode means the text cannot be trusted for structured parsing.
    let parsed = utf8_ok.then(|| parse_structured(&raw_lines)).flatten();
    let mode = match parsed {
        Some(_) => IniReadMode::Structured,
        None => IniReadMode::RawFallback,
    };
    let (variables, key_bindings) = parsed.unwrap_or_default();

    IniDocument {
        file_path: file_path.to_path_buf(),
        raw_lines,
        variables,
        key_bindings,
        had_bom,
        newline_style,
        mode,
    }
}

#[cfg(test)]
#[path = "tests/document_tests.rs"]
mod tests;
