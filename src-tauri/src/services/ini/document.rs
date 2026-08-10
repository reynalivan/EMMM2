//! Lossless INI read/discovery model for Epic 6.

use crate::domain::errors::AppError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub use super::encoding::{decode_ini_bytes, IniEncoding, LineTerminator};
use super::encoding::{decode_ini_source, source_fingerprint, split_lines_preserving_terminators};

static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\[([^\]]+)\]\s*$").expect("valid section regex"));
static VARIABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^\s*(?:(global|persist|local)\s+)?(\$[A-Za-z_][A-Za-z0-9_\.]*)\s*=\s*([^;#\r\n]+)",
    )
    .expect("valid variable regex")
});
static KEY_BACK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(key|back)\s*=\s*([^;#\r\n]+)").expect("valid key regex"));

/// Refuse to build an editable model for files this large — the editor holds
/// the whole document in memory and round-trips it on save.
const MAX_INI_BYTES: u64 = 2 * 1024 * 1024;

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
    pub qualifier: Option<String>,
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
    pub encoding: IniEncoding,
    pub newline_style: NewlineStyle,
    pub line_terminators: Vec<LineTerminator>,
    pub source_hash: String,
    pub mode: IniReadMode,
}

pub fn list_ini_files(mod_path: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !mod_path.exists() || !mod_path.is_dir() {
        return Err(AppError::Internal(format!(
            "Invalid mod path: {}",
            mod_path.display()
        )));
    }

    let mut entries = Vec::new();
    collect_ini_files(mod_path, mod_path, &mut entries)?;

    entries.sort_by_cached_key(|path| {
        path.strip_prefix(mod_path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase()
    });

    Ok(entries)
}

fn collect_ini_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        // Directory links and junction-like entries must not escape the mod root
        // or create recursion loops. File links are ignored for the same reason.
        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            let name = entry.file_name();
            if crate::common::normalizer::is_disabled_folder(&name.to_string_lossy()) {
                continue;
            }
            collect_ini_files(root, &path, output)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let is_ini = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"));
        let is_desktop_ini = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("desktop.ini"));
        if is_ini && !is_desktop_ini && path.starts_with(root) {
            output.push(path);
        }
    }

    Ok(())
}

/// Structured view of an INI body, or `None` when the file is not safely
/// parseable and the caller must fall back to raw lines.
fn parse_structured(raw_lines: &[String]) -> Option<(Vec<IniVariable>, Vec<KeyBinding>)> {
    let mut variables: Vec<IniVariable> = Vec::new();
    let mut key_bindings: Vec<KeyBinding> = Vec::new();
    let mut section_spellings: HashMap<String, String> = HashMap::new();
    let mut key_section: Option<String> = None;
    let mut section_binding_start = 0;

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
                    .map(|_| {
                        let logical_name = name.to_ascii_lowercase();
                        section_spellings
                            .entry(logical_name)
                            .or_insert_with(|| name.to_string())
                            .clone()
                    });
                section_binding_start = key_bindings.len();
                continue;
            }
        }

        if let Some(caps) = VARIABLE_RE.captures(line) {
            variables.push(IniVariable {
                qualifier: caps.get(1).map(|value| value.as_str().to_ascii_lowercase()),
                name: caps[2].trim().to_string(),
                value: caps[3].trim().to_string(),
                line_idx: idx,
            });
            continue;
        }

        let Some(section_name) = key_section.as_ref() else {
            continue;
        };

        let Some(caps) = KEY_BACK_RE.captures(line) else {
            continue;
        };
        let is_key = caps[1].eq_ignore_ascii_case("key");
        let entry_index = key_bindings[section_binding_start..]
            .iter()
            .rposition(|entry| {
                if is_key {
                    entry.key.is_none()
                } else {
                    entry.back.is_none()
                }
            })
            .map(|relative| section_binding_start + relative)
            .unwrap_or_else(|| {
                key_bindings.push(KeyBinding {
                    section_name: section_name.to_string(),
                    key: None,
                    back: None,
                    key_line_idx: None,
                    back_line_idx: None,
                });
                key_bindings.len() - 1
            });
        let entry = &mut key_bindings[entry_index];

        let value = caps[2].trim().to_string();
        if is_key {
            entry.key = Some(value);
            entry.key_line_idx = Some(idx);
        } else {
            entry.back = Some(value);
            entry.back_line_idx = Some(idx);
        }
    }

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
    let decoded = decode_ini_source(bytes);
    let (raw_lines, line_terminators) = split_lines_preserving_terminators(&decoded.text);
    let newline_style = if line_terminators.contains(&LineTerminator::CrLf) {
        NewlineStyle::CrLf
    } else {
        NewlineStyle::Lf
    };

    // A lossy decode means the text cannot be trusted for structured parsing.
    let parsed = decoded
        .clean
        .then(|| parse_structured(&raw_lines))
        .flatten();
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
        had_bom: decoded.had_bom,
        encoding: decoded.encoding,
        newline_style,
        line_terminators,
        source_hash: source_fingerprint(bytes),
        mode,
    }
}

#[cfg(test)]
#[path = "tests/document_tests.rs"]
mod tests;
