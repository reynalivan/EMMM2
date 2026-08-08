//! INI save pipeline with backup and atomic replace.

use crate::domain::errors::AppError;
use crate::services::ini::document::{IniDocument, IniReadMode, NewlineStyle};
use std::fs;
use std::path::{Path, PathBuf};

/// `foo.ini` + `bak` → `foo.ini.bak`, alongside the original.
fn sibling_with_suffix(file_path: &Path, suffix: &str) -> Result<PathBuf, AppError> {
    let file_name = file_path
        .file_name()
        .ok_or_else(|| AppError::Internal(format!("Invalid file path: {}", file_path.display())))?
        .to_string_lossy();
    Ok(file_path.with_file_name(format!("{}.{}", file_name, suffix)))
}

pub fn backup_path_for(file_path: &Path) -> Result<PathBuf, AppError> {
    sibling_with_suffix(file_path, "bak")
}

pub fn save_ini_with_updates(
    document: &IniDocument,
    line_updates: &[(usize, String)],
) -> Result<(), AppError> {
    if document.mode == IniReadMode::RawFallback {
        return Err(AppError::Internal(
            "Cannot save INI while parser is in raw fallback mode".to_string(),
        ));
    }

    let original_bytes = fs::read(&document.file_path)?;

    let backup_path = backup_path_for(&document.file_path)?;
    fs::write(&backup_path, &original_bytes)?;

    let mut lines = document.raw_lines.clone();
    for (line_idx, new_line) in line_updates {
        if *line_idx >= lines.len() {
            return Err(AppError::Internal(format!(
                "Line index out of bounds: {} (max {})",
                line_idx,
                lines.len().saturating_sub(1)
            )));
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

    let temp_path = sibling_with_suffix(&document.file_path, "tmp")?;
    fs::write(&temp_path, output)?;

    match fs::rename(&temp_path, &document.file_path) {
        Ok(_) => Ok(()),
        Err(_) => {
            // Windows refuses a rename onto an existing file; drop it and retry.
            if document.file_path.exists() {
                fs::remove_file(&document.file_path)?;
            }
            Ok(fs::rename(&temp_path, &document.file_path)?)
        }
    }
}

#[cfg(test)]
#[path = "tests/write_tests.rs"]
mod tests;
