//! Stale-safe INI save pipeline with recoverable replacement.

use crate::modules::library::application::ini::document::{IniDocument, IniReadMode};
use crate::modules::library::application::ini::encoding::{
    encode_ini_text, render_lines, source_fingerprint,
};
use crate::shared::errors::AppError;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
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

fn open_save_file(path: &Path, create_new: bool) -> std::io::Result<File> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create_new(create_new);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
            FILE_SHARE_READ,
        };
        options
            .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "INI save requires a regular non-linked file",
        ));
    }
    #[cfg(not(windows))]
    file.try_lock()?;
    Ok(file)
}

fn rename_save_file(
    file: &File,
    source: &Path,
    target: &Path,
    parent: &crate::platform::fs::rename::PinnedDirectory,
) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let _ = source;
        crate::platform::fs::rename::rename_handle_into(
            file,
            parent,
            target.file_name().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid INI destination")
            })?,
        )
    }
    #[cfg(not(windows))]
    {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let expected = file.metadata()?;
            let actual = fs::symlink_metadata(source)?;
            if (expected.dev(), expected.ino()) != (actual.dev(), actual.ino()) {
                return Err(std::io::Error::other(
                    "INI source ownership changed during save",
                ));
            }
        }
        crate::platform::fs::rename::rename_no_replace_into(
            source,
            parent,
            target.file_name().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid INI destination")
            })?,
        )
    }
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
    source_file: &File,
    parent: &crate::platform::fs::rename::PinnedDirectory,
) -> Result<(), AppError> {
    match rename_save_file(source_file, recovery, target, parent) {
        Ok(()) => {
            if let Err(cleanup_error) = fs::remove_file(temp) {
                log::warn!(
                    "Failed to remove rejected INI temp {}: {cleanup_error}",
                    temp.display()
                );
            }
            Err(crate::platform::fs::retry::map_file_error(
                target,
                "commit INI (original restored)",
                commit_error,
            ))
        }
        Err(restore_error) => Err(AppError::Io(format!(
            "INI commit failed ({commit_error}); automatic restore also failed ({restore_error}). Recovery file: {}",
            recovery.display()
        ))),
    }
}

fn finish_ini_commit(
    source_file: &File,
    temp_file: File,
    target: &Path,
    backup: &Path,
    temp: &Path,
    parent: &crate::platform::fs::rename::PinnedDirectory,
    backup_validation: std::io::Result<()>,
) -> Result<(), AppError> {
    if let Err(error) = backup_validation {
        drop(temp_file);
        return restore_after_failed_commit(target, backup, temp, error, source_file, parent);
    }
    if let Err(error) = rename_save_file(&temp_file, temp, target, parent) {
        drop(temp_file);
        return restore_after_failed_commit(target, backup, temp, error, source_file, parent);
    }
    Ok(())
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

    let parent_path = document
        .file_path
        .parent()
        .ok_or_else(|| AppError::Validation("INI has no parent directory".to_string()))?;
    let parent = crate::platform::fs::rename::PinnedDirectory::open(parent_path)?;
    let file_path = parent.stable_path().join(
        document
            .file_path
            .file_name()
            .ok_or_else(|| AppError::Validation("INI has no filename".to_string()))?,
    );
    let current_bytes = fs::read(&file_path).map_err(|error| {
        crate::platform::fs::retry::map_file_error(&file_path, "read INI before save", error)
    })?;
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
    if output == current_bytes {
        return Ok(());
    }

    let mut source_file = open_save_file(&file_path, false).map_err(|error| {
        crate::platform::fs::retry::map_file_error(&file_path, "acquire INI save ownership", error)
    })?;
    let mut protected_bytes = Vec::new();
    source_file
        .read_to_end(&mut protected_bytes)
        .map_err(|error| {
            crate::platform::fs::retry::map_file_error(&file_path, "read protected INI", error)
        })?;
    if source_fingerprint(&protected_bytes) != current_hash {
        return Err(AppError::Validation(
            "INI changed before save ownership could be acquired; reload it before saving"
                .to_string(),
        ));
    }

    let temp_path = unique_sibling(&file_path, "tmp")?;
    let mut temp_file = open_save_file(&temp_path, true)?;
    temp_file.write_all(&output)?;
    temp_file.sync_all()?;

    rotate_backups(&file_path)?;
    let backup_path = backup_path_for(&file_path)?;
    source_file.sync_all()?;
    if let Err(error) = rename_save_file(&source_file, &file_path, &backup_path, &parent) {
        drop(temp_file);
        if let Err(cleanup_error) = fs::remove_file(&temp_path) {
            log::warn!(
                "Failed to remove rejected INI temp {}: {cleanup_error}",
                temp_path.display()
            );
        }
        return Err(crate::platform::fs::retry::map_file_error(
            &file_path,
            "retain INI backup",
            error,
        ));
    }

    // Retain the original inode as the backup, including writes through old
    // handles on platforms where file locks are advisory.
    let validation = fs::read(&backup_path).and_then(|bytes| {
        if source_fingerprint(&bytes) == current_hash {
            Ok(())
        } else {
            Err(std::io::Error::other(
                "INI changed during save; concurrent bytes preserved",
            ))
        }
    });
    finish_ini_commit(
        &source_file,
        temp_file,
        &file_path,
        &backup_path,
        &temp_path,
        &parent,
        validation,
    )
}

#[cfg(test)]
#[path = "tests/write_tests.rs"]
mod tests;
