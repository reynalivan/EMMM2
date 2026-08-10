use crate::domain::errors::AppError;
use std::path::Path;

pub fn move_path_to_recycle_bin(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    trash::delete(path).map_err(|error| {
        AppError::Io(format!(
            "Failed to move '{}' to the system Recycle Bin: {error}",
            path.display()
        ))
    })
}
