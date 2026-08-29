use crate::domain::errors::AppError;
use std::path::{Path, PathBuf};

pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    crate::services::fs_utils::atomic_file::atomic_write(path, content.as_bytes())
}

pub fn create_staging_directory(active: &Path) -> Result<PathBuf, AppError> {
    crate::services::fs_utils::atomic_file::create_staging_directory(active)
}

pub fn replace_directory(staging: &Path, active: &Path) -> Result<(), AppError> {
    crate::services::fs_utils::atomic_file::replace_directory(staging, active)
}
