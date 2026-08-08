use std::fs;
use std::path::Path;

/// OS error codes that mean "the rename crossed a volume boundary":
/// Windows `ERROR_NOT_SAME_DEVICE` (17) and Unix `EXDEV` (18).
///
/// The overlap is not clean — on Unix 17 is `EEXIST`, on Windows 18 is
/// `ERROR_NO_MORE_FILES` — so this predicate is deliberately broad and the
/// fallback re-checks its own preconditions before copying anything.
const CROSS_DEVICE_ERROR_CODES: [i32; 2] = [17, 18];

fn is_cross_device_error(error: &std::io::Error) -> bool {
    error
        .raw_os_error()
        .is_some_and(|code| CROSS_DEVICE_ERROR_CODES.contains(&code))
}

/// Tries to rename a file or directory using `std::fs::rename`.
/// If it fails (likely due to cross-device link errors), it falls back
/// to using `fs_extra` to copy and remove the original.
///
/// Callers should invoke this directly — it already attempts the plain rename
/// first, so wrapping it in another `fs::rename(..).or_else(..)` just retries.
///
/// # Covers: Cross-drive `fs::rename` fallback (copy/delete)
pub fn rename_cross_drive_fallback(from: &Path, to: &Path) -> std::io::Result<()> {
    let Err(error) = fs::rename(from, to) else {
        return Ok(());
    };

    if !is_cross_device_error(&error) {
        return Err(error);
    }

    log::warn!(
        "fs::rename failed (cross-device?): {}. Attempting fallback move...",
        error
    );

    if !from.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Source path does not exist",
        ));
    }

    if to.exists() {
        return Err(error); // Propagate the original error (e.g., AlreadyExists)
    }

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }

    // `CopyOptions::new()` already defaults to no-overwrite / no-copy-inside.
    let moved = if from.is_dir() {
        fs_extra::dir::move_dir(from, to, &fs_extra::dir::CopyOptions::new())
    } else {
        fs_extra::file::move_file(from, to, &fs_extra::file::CopyOptions::new())
    };

    moved
        .map(|_| ())
        .map_err(|err| std::io::Error::other(err.to_string()))
}
