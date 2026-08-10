//! Stale-safe INI save pipeline with recoverable replacement.

use crate::domain::errors::AppError;
use crate::services::ini::document::{IniDocument, IniReadMode};
use crate::services::ini::encoding::{encode_ini_text, render_lines, source_fingerprint};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const BACKUP_GENERATIONS: usize = 3;
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn sibling_with_suffix(file_path: &Path, suffix: &str) -> Result<PathBuf, AppError> {
    let file_name = file_path
        .file_name()
        .ok_or_else(|| AppError::Internal(format!("Invalid file path: {}", file_path.display())))?
        .to_string_lossy();
    Ok(file_path.with_file_name(format!("{file_name}.{suffix}")))
}

fn unique_sibling(file_path: &Path, label: &str) -> Result<PathBuf, AppError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Internal(format!("System clock error: {error}")))?
        .as_nanos();
    let sequence = WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    sibling_with_suffix(
        file_path,
        &format!("{label}.{}.{}.{}", std::process::id(), timestamp, sequence),
    )
}

pub fn backup_path_for(file_path: &Path) -> Result<PathBuf, AppError> {
    sibling_with_suffix(file_path, "bak")
}

fn numbered_backup_path(file_path: &Path, generation: usize) -> Result<PathBuf, AppError> {
    sibling_with_suffix(file_path, &format!("bak.{generation}"))
}

fn rotate_backups(file_path: &Path) -> Result<(), AppError> {
    for generation in (1..=BACKUP_GENERATIONS).rev() {
        let source = if generation == 1 {
            backup_path_for(file_path)?
        } else {
            numbered_backup_path(file_path, generation - 1)?
        };
        if !source.exists() {
            continue;
        }

        let target = numbered_backup_path(file_path, generation)?;
        if target.exists() {
            fs::remove_file(&target)?;
        }
        fs::rename(source, target)?;
    }
    Ok(())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn validate_updates(
    document: &IniDocument,
    line_updates: &[(usize, String)],
) -> Result<(), AppError> {
    let mut seen_indices = HashSet::new();
    for (line_idx, new_line) in line_updates {
        if *line_idx >= document.raw_lines.len() {
            return Err(AppError::Validation(format!(
                "Line index out of bounds: {line_idx} (max {})",
                document.raw_lines.len().saturating_sub(1)
            )));
        }
        if !seen_indices.insert(*line_idx) {
            return Err(AppError::Validation(format!(
                "Duplicate update for INI line {line_idx}"
            )));
        }
        if new_line.contains(['\r', '\n']) {
            return Err(AppError::Validation(format!(
                "INI line {line_idx} must not contain a newline"
            )));
        }
    }
    Ok(())
}

fn restore_after_failed_commit(
    target: &Path,
    recovery: &Path,
    temp: &Path,
    commit_error: std::io::Error,
) -> Result<(), AppError> {
    match fs::rename(recovery, target) {
        Ok(()) => {
            if let Err(cleanup_error) = fs::remove_file(temp) {
                log::warn!(
                    "Failed to remove rejected INI temp {}: {cleanup_error}",
                    temp.display()
                );
            }
            Err(AppError::Io(format!(
                "INI commit failed and the original was restored: {commit_error}"
            )))
        }
        Err(restore_error) => Err(AppError::Io(format!(
            "INI commit failed ({commit_error}); automatic restore also failed ({restore_error}). Recovery file: {}",
            recovery.display()
        ))),
    }
}

pub fn save_ini_with_updates(
    document: &IniDocument,
    expected_source_hash: &str,
    line_updates: &[(usize, String)],
) -> Result<(), AppError> {
    if document.mode == IniReadMode::RawFallback {
        return Err(AppError::Validation(
            "Cannot save INI while parser is in raw fallback mode".to_string(),
        ));
    }
    validate_updates(document, line_updates)?;
    if line_updates.is_empty() {
        return Ok(());
    }

    let current_bytes = fs::read(&document.file_path)?;
    let current_hash = source_fingerprint(&current_bytes);
    if current_hash != expected_source_hash || current_hash != document.source_hash {
        return Err(AppError::Validation(
            "INI changed on disk after it was opened; reload it before saving".to_string(),
        ));
    }

    let mut lines = document.raw_lines.clone();
    for (line_idx, new_line) in line_updates {
        lines[*line_idx] = new_line.clone();
    }
    let text = render_lines(&lines, &document.line_terminators)?;
    let output = encode_ini_text(&text, document.encoding, document.had_bom)?;

    let temp_path = unique_sibling(&document.file_path, "tmp")?;
    write_synced(&temp_path, &output)?;

    rotate_backups(&document.file_path)?;
    let backup_path = backup_path_for(&document.file_path)?;
    write_synced(&backup_path, &current_bytes)?;

    let recovery_path = unique_sibling(&document.file_path, "recover")?;
    if let Err(error) = fs::rename(&document.file_path, &recovery_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }

    if let Err(error) = fs::rename(&temp_path, &document.file_path) {
        return restore_after_failed_commit(&document.file_path, &recovery_path, &temp_path, error);
    }

    if let Err(error) = fs::remove_file(&recovery_path) {
        log::warn!(
            "INI save committed but old recovery file {} could not be removed: {error}",
            recovery_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/write_tests.rs"]
mod tests;
