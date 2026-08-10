use crate::domain::errors::AppError;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn unique_sibling(path: &Path, label: &str) -> Result<PathBuf, AppError> {
    let name = path
        .file_name()
        .ok_or_else(|| AppError::Validation(format!("Invalid artifact path: {}", path.display())))?
        .to_string_lossy();
    let id = ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!("{name}.{label}.{}.{id}", std::process::id())))
}

fn restore_file(target: &Path, recovery: &Path, commit_error: std::io::Error) -> AppError {
    match fs::rename(recovery, target) {
        Ok(()) => AppError::Io(format!(
            "Artifact replace failed and the previous file was restored: {commit_error}"
        )),
        Err(restore_error) => AppError::Io(format!(
            "Artifact replace failed ({commit_error}); restore failed ({restore_error}). Recovery: {}",
            recovery.display()
        )),
    }
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp = unique_sibling(path, "tmp")?;
    let mut file = File::create(&temp)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;

    if !path.exists() {
        return Ok(fs::rename(temp, path)?);
    }

    let recovery = unique_sibling(path, "recover")?;
    fs::rename(path, &recovery)?;
    if let Err(error) = fs::rename(&temp, path) {
        return Err(restore_file(path, &recovery, error));
    }
    if let Err(error) = fs::remove_file(&recovery) {
        log::warn!(
            "Artifact committed but recovery file {} remains: {error}",
            recovery.display()
        );
    }
    Ok(())
}

pub fn create_staging_directory(active: &Path) -> Result<PathBuf, AppError> {
    let staging = unique_sibling(active, "staging")?;
    fs::create_dir_all(&staging)?;
    Ok(staging)
}

pub fn replace_directory(staging: &Path, active: &Path) -> Result<(), AppError> {
    if !staging.is_dir() {
        return Err(AppError::Validation(format!(
            "Artifact staging directory is missing: {}",
            staging.display()
        )));
    }
    if !active.exists() {
        return Ok(fs::rename(staging, active)?);
    }

    let recovery = unique_sibling(active, "recover")?;
    fs::rename(active, &recovery)?;
    if let Err(commit_error) = fs::rename(staging, active) {
        return match fs::rename(&recovery, active) {
            Ok(()) => Err(AppError::Io(format!(
                "Artifact directory replace failed and previous artifacts were restored: {commit_error}"
            ))),
            Err(restore_error) => Err(AppError::Io(format!(
                "Artifact directory replace failed ({commit_error}); restore failed ({restore_error}). Recovery: {}",
                recovery.display()
            ))),
        };
    }
    if let Err(error) = fs::remove_dir_all(&recovery) {
        log::warn!(
            "Artifact directory committed but recovery {} remains: {error}",
            recovery.display()
        );
    }
    Ok(())
}
