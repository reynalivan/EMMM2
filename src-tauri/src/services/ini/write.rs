//! INI save pipeline with backup and atomic replace.

use crate::services::ini::document::{IniDocument, IniReadMode, NewlineStyle};
use std::fs;
use std::path::{Path, PathBuf};

pub fn backup_path_for(file_path: &Path) -> Result<PathBuf, String> {
    let file_name = file_path
        .file_name()
        .ok_or_else(|| format!("Invalid file path: {}", file_path.display()))?
        .to_string_lossy();
    Ok(file_path.with_file_name(format!("{}.bak", file_name)))
}

fn temp_path_for(file_path: &Path) -> Result<PathBuf, String> {
    let file_name = file_path
        .file_name()
        .ok_or_else(|| format!("Invalid file path: {}", file_path.display()))?
        .to_string_lossy();
    Ok(file_path.with_file_name(format!("{}.tmp", file_name)))
}

pub fn save_ini_with_updates(
    document: &IniDocument,
    line_updates: &[(usize, String)],
) -> Result<(), String> {
    if document.mode == IniReadMode::RawFallback {
        return Err("Cannot save INI while parser is in raw fallback mode".to_string());
    }

    let original_bytes = fs::read(&document.file_path)
        .map_err(|e| format!("Failed to read original INI bytes: {e}"))?;

    let backup_path = backup_path_for(&document.file_path)?;
    fs::write(&backup_path, &original_bytes)
        .map_err(|e| format!("Failed to write backup file: {e}"))?;

    let mut lines = document.raw_lines.clone();
    for (line_idx, new_line) in line_updates {
        if *line_idx >= lines.len() {
            return Err(format!(
                "Line index out of bounds: {} (max {})",
                line_idx,
                lines.len().saturating_sub(1)
            ));
        }
        lines[*line_idx] = new_line.clone();
    }

    let separator = match document.newline_style {
        NewlineStyle::CrLf => "\r\n",
        NewlineStyle::Lf => "\n",
    };

    let mut output = lines.join(separator).into_bytes();
    if document.had_bom {
        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.extend_from_slice(&output);
        output = with_bom;
    }

    let temp_path = temp_path_for(&document.file_path)?;
    fs::write(&temp_path, output).map_err(|e| format!("Failed to write temp INI file: {e}"))?;

    match fs::rename(&temp_path, &document.file_path) {
        Ok(_) => Ok(()),
        Err(_) => {
            if document.file_path.exists() {
                fs::remove_file(&document.file_path)
                    .map_err(|e| format!("Failed to replace INI target file: {e}"))?;
            }
            fs::rename(&temp_path, &document.file_path)
                .map_err(|e| format!("Failed to finalize INI write: {e}"))
        }
    }
}

#[cfg(test)]
#[path = "tests/write_tests.rs"]
mod tests;
