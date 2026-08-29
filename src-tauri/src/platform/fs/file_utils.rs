use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// OS error codes that mean "the rename crossed a volume boundary":
/// Windows `ERROR_NOT_SAME_DEVICE` (17) and Unix `EXDEV` (18).
///
/// The overlap is not clean — on Unix 17 is `EEXIST`, on Windows 18 is
/// `ERROR_NO_MORE_FILES` — so this predicate is deliberately broad and the
/// fallback re-checks its own preconditions before copying anything.
const CROSS_DEVICE_ERROR_CODES: [i32; 2] = [17, 18];

#[derive(Debug)]
pub struct TrackedMoveError {
    error: io::Error,
    target_owned: bool,
}

impl TrackedMoveError {
    fn unchanged(error: io::Error) -> Self {
        Self {
            error,
            target_owned: false,
        }
    }

    fn target_owned(error: io::Error) -> Self {
        Self {
            error,
            target_owned: true,
        }
    }

    pub fn target_is_owned(&self) -> bool {
        self.target_owned
    }

    pub fn into_io_error(self) -> io::Error {
        self.error
    }
}

fn is_cross_device_error(error: &io::Error) -> bool {
    error
        .raw_os_error()
        .is_some_and(|code| CROSS_DEVICE_ERROR_CODES.contains(&code))
}

/// Tries to rename a file or directory using `std::fs::rename`.
/// If the source and target are on different volumes, it copies into a
/// target-volume temporary path, publishes that path at the exact requested
/// destination, and only then retires the source.
///
/// An error with `target_is_owned() == true` means the fallback published the
/// destination but could not restore the pre-move state. Journal-aware callers
/// must include that destination in rollback.
pub fn rename_cross_drive_fallback_tracked(from: &Path, to: &Path) -> Result<(), TrackedMoveError> {
    let Err(error) = fs::rename(from, to) else {
        return Ok(());
    };

    if !is_cross_device_error(&error) {
        return Err(TrackedMoveError::unchanged(error));
    }

    log::warn!(
        "fs::rename failed across a volume boundary: {error}. Attempting staged fallback move"
    );

    if !from.exists() {
        return Err(TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::NotFound,
            "Source path does not exist",
        )));
    }

    if to.exists() {
        return Err(TrackedMoveError::unchanged(error));
    }

    let parent = to.parent().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent",
        ))
    })?;
    fs::create_dir_all(parent).map_err(TrackedMoveError::unchanged)?;

    if from.is_dir() {
        move_directory_across_volumes(from, to, parent)
    } else {
        move_file_across_volumes(from, to, parent)
    }
}

/// Convenience wrapper for callers that do not maintain a move journal.
pub fn rename_cross_drive_fallback(from: &Path, to: &Path) -> io::Result<()> {
    rename_cross_drive_fallback_tracked(from, to).map_err(TrackedMoveError::into_io_error)
}

fn move_directory_across_volumes(
    from: &Path,
    to: &Path,
    target_parent: &Path,
) -> Result<(), TrackedMoveError> {
    let staging = tempfile::Builder::new()
        .prefix(".emmm_move_")
        .tempdir_in(target_parent)
        .map_err(TrackedMoveError::unchanged)?;
    let options = fs_extra::dir::CopyOptions::new()
        .copy_inside(true)
        .content_only(true);

    if let Err(copy_error) = fs_extra::dir::copy(from, staging.path(), &options) {
        let copy_error = io::Error::other(copy_error.to_string());
        return match staging.close() {
            Ok(()) => Err(TrackedMoveError::unchanged(copy_error)),
            Err(cleanup_error) => Err(TrackedMoveError::unchanged(io::Error::other(format!(
                "Cross-volume directory copy failed: {copy_error}; temporary cleanup failed: {cleanup_error}"
            )))),
        };
    }

    fs::rename(staging.path(), to).map_err(TrackedMoveError::unchanged)?;
    drop(staging);
    retire_source_or_remove_published_target(from, to)
}

fn move_file_across_volumes(
    from: &Path,
    to: &Path,
    target_parent: &Path,
) -> Result<(), TrackedMoveError> {
    let staging =
        tempfile::NamedTempFile::new_in(target_parent).map_err(TrackedMoveError::unchanged)?;
    fs::copy(from, staging.path()).map_err(TrackedMoveError::unchanged)?;
    staging
        .as_file()
        .sync_all()
        .map_err(TrackedMoveError::unchanged)?;
    staging
        .persist_noclobber(to)
        .map_err(|error| TrackedMoveError::unchanged(error.error))?;
    retire_source_or_remove_published_target(from, to)
}

fn retire_source_or_remove_published_target(
    from: &Path,
    to: &Path,
) -> Result<(), TrackedMoveError> {
    let tombstone = source_tombstone_path(from);
    if let Err(retire_error) = fs::rename(from, &tombstone) {
        return match remove_path(to) {
            Ok(()) => Err(TrackedMoveError::unchanged(retire_error)),
            Err(cleanup_error) => Err(TrackedMoveError::target_owned(io::Error::other(format!(
                "Could not retire source after publishing destination: {retire_error}; destination cleanup failed: {cleanup_error}"
            )))),
        };
    }

    if let Err(error) = remove_path(&tombstone) {
        // The visible move is already complete and the destination contains a
        // full copy. Keep the recoverable hidden source residue instead of
        // reporting a failed move that a caller might retry into a collision.
        log::warn!(
            "Cross-volume move completed, but source tombstone '{}' could not be removed: {error}",
            tombstone.display()
        );
    }
    Ok(())
}

fn source_tombstone_path(source: &Path) -> PathBuf {
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source");
    parent.join(format!(
        ".emmm_move_source_{}_{}",
        name,
        uuid::Uuid::new_v4()
    ))
}

fn remove_path(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{move_directory_across_volumes, move_file_across_volumes};

    #[test]
    fn directory_fallback_publishes_the_exact_requested_shape() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("archive-root");
        let target_parent = root.path().join("Ayaka");
        let target = target_parent.join("DISABLED Skin");
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::create_dir_all(&target_parent).unwrap();
        std::fs::write(source.join("nested/mod.ini"), "[TextureOverride]").unwrap();

        move_directory_across_volumes(&source, &target, &target_parent).unwrap();

        assert!(target.join("nested/mod.ini").is_file());
        assert!(!target.join("archive-root").exists());
        assert!(!source.exists());
    }

    #[test]
    fn file_fallback_publishes_the_exact_requested_path() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.zip");
        let target_parent = root.path().join("Processed");
        let target = target_parent.join("source.zip");
        std::fs::create_dir(&target_parent).unwrap();
        std::fs::write(&source, "archive").unwrap();

        move_file_across_volumes(&source, &target, &target_parent).unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "archive");
        assert!(!source.exists());
    }
}
