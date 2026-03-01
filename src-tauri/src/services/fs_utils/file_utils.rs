use std::fs;
use std::path::Path;

/// Tries to rename a file or directory using `std::fs::rename`.
/// If it fails (likely due to cross-device link errors), it falls back
/// to using `fs_extra` to copy and remove the original.
///
/// # Covers: Cross-drive `fs::rename` fallback (copy/delete)
pub fn rename_cross_drive_fallback(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!(
                "fs::rename failed (cross-device?): {}. Attempting fallback move...",
                e
            );

            if !from.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Source path does not exist",
                ));
            }

            if to.exists() {
                return Err(e); // Propagate the original error (e.g., AlreadyExists)
            }

            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }

            if from.is_dir() {
                let mut options = fs_extra::dir::CopyOptions::new();
                options.copy_inside = false;

                fs_extra::dir::move_dir(from, to, &options)
                    .map(|_| ())
                    .map_err(|err| std::io::Error::other(err.to_string()))
            } else {
                let mut options = fs_extra::file::CopyOptions::new();
                options.overwrite = false;

                fs_extra::file::move_file(from, to, &options)
                    .map(|_| ())
                    .map_err(|err| std::io::Error::other(err.to_string()))
            }
        }
    }
}
