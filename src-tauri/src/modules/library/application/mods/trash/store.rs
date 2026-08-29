//! Native recycle-bin adapter.

use crate::shared::errors::AppError;
use std::path::Path;

pub fn move_to_trash(source_path: &Path) -> Result<(), AppError> {
    if !source_path.exists() {
        return Err(AppError::Io(format!(
            "Source does not exist: {}",
            source_path.display()
        )));
    }
    if !source_path.is_dir() {
        return Err(AppError::Io("Only directories can be trashed".to_string()));
    }

    crate::platform::fs::recycle_bin::move_path_to_recycle_bin(source_path)?;
    log::info!("Moved '{}' to the OS recycle bin", source_path.display());
    Ok(())
}
