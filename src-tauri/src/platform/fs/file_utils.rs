use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn filesystem_identity(path: &Path) -> Option<String> {
    Some(match file_id::get_file_id(path).ok()? {
        file_id::FileId::Inode {
            device_id,
            inode_number,
        } => format!("inode:{device_id}:{inode_number}"),
        file_id::FileId::LowRes {
            volume_serial_number,
            file_index,
        } => format!("win-low:{volume_serial_number}:{file_index}"),
        file_id::FileId::HighRes {
            volume_serial_number,
            file_id,
        } => format!("win-high:{volume_serial_number}:{file_id}"),
    })
}

/// Retain pre-wait ownership for immutable writers without rebinding a path
/// to a replacement folder after acquiring the mutation lease.
#[derive(Debug, Clone)]
pub(crate) struct FilesystemIdentityProof {
    identity: String,
}

impl FilesystemIdentityProof {
    pub(crate) fn capture(path: &Path) -> Result<Self, crate::shared::errors::AppError> {
        let identity = filesystem_identity(path).ok_or_else(|| {
            crate::shared::errors::AppError::Io(format!(
                "Cannot verify filesystem identity at {}; refresh before retrying",
                path.display()
            ))
        })?;
        Ok(Self { identity })
    }

    pub(crate) fn identity(&self) -> &str {
        &self.identity
    }

    pub(crate) fn validate(&self, path: &Path) -> Result<(), crate::shared::errors::AppError> {
        if filesystem_identity(path).as_deref() != Some(self.identity()) {
            return Err(crate::shared::errors::AppError::Io(format!(
                "Selected filesystem entry changed at {}; refresh before retrying",
                path.display()
            )));
        }
        Ok(())
    }
}

/// Request-local ancestry ownership for sibling renames. Source spelling is
/// deliberately excluded so a preceding enabled/DISABLED rename stays valid.
#[derive(Debug, Clone)]
pub(crate) struct FilesystemNamespaceProof {
    root: PathBuf,
    parents: std::collections::BTreeMap<PathBuf, FilesystemIdentityProof>,
}

impl FilesystemNamespaceProof {
    pub(crate) fn capture(
        root: &Path,
        paths: &[PathBuf],
    ) -> Result<Self, crate::shared::errors::AppError> {
        let root = crate::shared::path_key::physical_namespace_path(root)?;
        let mut proof = Self {
            root,
            parents: std::collections::BTreeMap::new(),
        };
        for path in paths {
            for parent in proof.parent_chain(path)? {
                if !proof.parents.contains_key(&parent) {
                    proof
                        .parents
                        .insert(parent.clone(), FilesystemIdentityProof::capture(&parent)?);
                }
            }
        }
        proof.validate_paths(paths)?;
        Ok(proof)
    }

    fn parent_chain(&self, path: &Path) -> Result<Vec<PathBuf>, crate::shared::errors::AppError> {
        let path = crate::shared::path_key::physical_namespace_path(path)?;
        if path == self.root || !path.starts_with(&self.root) {
            return Err(crate::shared::errors::AppError::Security(
                "Toggle must remain below its captured Mods root".into(),
            ));
        }
        let mut parents = Vec::new();
        for parent in path.parent().into_iter().flat_map(Path::ancestors) {
            parents.push(parent.to_path_buf());
            if parent == self.root {
                break;
            }
        }
        Ok(parents)
    }

    pub(crate) fn validate_paths(
        &self,
        paths: &[PathBuf],
    ) -> Result<(), crate::shared::errors::AppError> {
        let mut checked = std::collections::BTreeSet::new();
        for path in paths {
            for parent in self.parent_chain(path)? {
                if !checked.insert(parent.clone()) {
                    continue;
                }
                let proof = self.parents.get(&parent).ok_or_else(|| {
                    crate::shared::errors::AppError::Io(
                        "Toggle ancestry is not covered by its original request".into(),
                    )
                })?;
                proof.validate(&parent)?;
                let canonical = parent.canonicalize()?;
                if crate::shared::path_key::physical_namespace_path(&canonical)? != parent {
                    return Err(crate::shared::errors::AppError::Security(
                        "Toggle ancestor binding changed; refresh before retrying".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Retain the original ancestry while a native directory mutation runs.
    pub(crate) fn pin_paths(
        &self,
        paths: &[PathBuf],
    ) -> Result<Vec<super::rename::PinnedDirectory>, crate::shared::errors::AppError> {
        self.validate_paths(paths)?;
        let mut pins = Vec::new();
        let mut checked = std::collections::BTreeSet::new();
        for path in paths {
            for parent in self.parent_chain(path)? {
                if !checked.insert(parent.clone()) {
                    continue;
                }
                let pin = super::rename::PinnedDirectory::open(&parent)?;
                if self
                    .parents
                    .get(&parent)
                    .map(FilesystemIdentityProof::identity)
                    != Some(pin.identity())
                {
                    return Err(crate::shared::errors::AppError::Io(
                        "Mutation ancestry changed while acquiring ownership; refresh before retrying"
                            .into(),
                    ));
                }
                pins.push(pin);
            }
        }
        self.validate_paths(paths)?;
        Ok(pins)
    }
}

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
    target_proof: Option<FilesystemIdentityProof>,
    target_path: Option<PathBuf>,
}

impl TrackedMoveError {
    fn unchanged(error: io::Error) -> Self {
        Self {
            error,
            target_proof: None,
            target_path: None,
        }
    }

    fn target_owned(error: io::Error, path: &Path, proof: &FilesystemIdentityProof) -> Self {
        Self {
            error,
            target_proof: Some(proof.clone()),
            target_path: Some(path.to_path_buf()),
        }
    }

    pub fn target_is_owned(&self) -> bool {
        self.target_proof.is_some()
    }

    pub(crate) fn target_proof(&self) -> Option<&FilesystemIdentityProof> {
        self.target_proof.as_ref()
    }

    pub(crate) fn target_path(&self) -> Option<&Path> {
        self.target_path.as_deref()
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

/// Tries an atomic no-overwrite rename of a file or directory.
/// If the source and target are on different volumes, it copies into a
/// target-volume temporary path, publishes that path at the exact requested
/// destination, and only then retires the source.
///
/// An error with `target_is_owned() == true` means the fallback published the
/// destination but could not restore the pre-move state. Journal-aware callers
/// must include that destination in rollback.
pub(crate) fn rename_cross_drive_fallback_tracked(
    from: &Path,
    to: &Path,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    let parent = to.parent().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent",
        ))
    })?;
    rename_cross_drive_fallback_tracked_with(from, to, parent, &|source| {
        super::rename::rename_no_replace(source, to)
    })
}

#[cfg(test)]
pub(crate) fn rename_cross_drive_fallback_tracked_into(
    from: &Path,
    to: &Path,
    destination_parent: &super::rename::PinnedDirectory,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    rename_cross_drive_fallback_tracked_into_with_hook(
        from,
        to,
        destination_parent,
        None,
        || {},
        || {},
    )
}

pub(crate) fn rename_cross_drive_fallback_owned_into(
    from: &Path,
    to: &Path,
    destination_parent: &super::rename::PinnedDirectory,
    expected_source: &FilesystemIdentityProof,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    rename_cross_drive_fallback_tracked_into_with_hook(
        from,
        to,
        destination_parent,
        Some(expected_source),
        || {},
        || {},
    )
}

struct OwnedDirectorySource<'a> {
    directory: super::rename::PinnedDirectory,
    parent: &'a super::rename::PinnedDirectory,
    proof: FilesystemIdentityProof,
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn rename_cross_drive_fallback_tracked_into_with_test_hook(
    from: &Path,
    to: &Path,
    destination_parent: &super::rename::PinnedDirectory,
    before_compensation: impl FnOnce(),
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    rename_cross_drive_fallback_tracked_into_with_hook(
        from,
        to,
        destination_parent,
        None,
        || {},
        before_compensation,
    )
}

fn rename_cross_drive_fallback_tracked_into_with_hook(
    from: &Path,
    to: &Path,
    destination_parent: &super::rename::PinnedDirectory,
    expected_source: Option<&FilesystemIdentityProof>,
    before_source_binding: impl FnOnce(),
    before_compensation: impl FnOnce(),
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    if let Some(proof) = expected_source {
        proof
            .validate(from)
            .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    }
    before_source_binding();
    let source_name = from.file_name().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source path has no file name",
        ))
    })?;
    let source_parent_path = from.parent().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source path has no parent",
        ))
    })?;
    let source_parent = super::rename::PinnedDirectory::open(source_parent_path)
        .map_err(TrackedMoveError::unchanged)?;
    let stable_source_parent = source_parent.stable_path();
    let stable_source = stable_source_parent.join(source_name);
    let owned_source = expected_source
        .map(|proof| {
            let directory = super::rename::PinnedDirectory::open_for_rename(&stable_source)
                .map_err(TrackedMoveError::unchanged)?;
            if directory.identity() != proof.identity() {
                return Err(TrackedMoveError::unchanged(io::Error::other(
                    "Import source changed while binding cross-volume publication",
                )));
            }
            Ok(OwnedDirectorySource {
                directory,
                parent: &source_parent,
                proof: proof.clone(),
            })
        })
        .transpose()?;
    let destination_name = to.file_name().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no file name",
        ))
    })?;
    let requested_parent = to.parent().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent",
        ))
    })?;
    let requested_parent = crate::shared::path_key::physical_namespace_path(requested_parent)
        .map_err(TrackedMoveError::unchanged)?;
    let pinned_parent = crate::shared::path_key::physical_namespace_path(destination_parent.path())
        .map_err(TrackedMoveError::unchanged)?;
    if requested_parent != pinned_parent {
        return Err(TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Pinned destination parent does not match the requested target",
        )));
    }
    let stable_parent = destination_parent.stable_path();
    let stable_target = stable_parent.join(destination_name);
    let result = rename_cross_drive_fallback_tracked_with_source(
        from,
        &stable_target,
        &stable_parent,
        owned_source,
        &|source, owned_source| match owned_source {
            Some(source) => source.rename_into(destination_parent, destination_name),
            None => {
                super::rename::rename_no_replace_into(source, destination_parent, destination_name)
            }
        },
    );
    let target_proof = match result {
        Ok(target_proof) => target_proof,
        Err(mut error) => {
            if error.target_proof.is_some() {
                error.target_path = destination_parent
                    .current_path()
                    .ok()
                    .map(|parent| parent.join(destination_name));
            }
            return Err(error);
        }
    };
    if target_proof.validate(to).is_ok() {
        return Ok(target_proof);
    }

    let binding_error = io::Error::other(format!(
        "Import destination namespace changed after publication: {}",
        to.display()
    ));
    before_compensation();
    match rename_cross_drive_fallback_tracked_with(
        &stable_target,
        &stable_source,
        &stable_source_parent,
        &|source| super::rename::rename_no_replace_into(source, &source_parent, source_name),
    ) {
        Ok(_) => Err(TrackedMoveError::unchanged(binding_error)),
        Err(restoration_error) => Err(classify_compensation_failure(
            binding_error,
            restoration_error,
            CompensationContext {
                original_target_proof: &target_proof,
                stable_target: &stable_target,
                destination_parent,
                destination_name,
                stable_source: &stable_source,
                source_parent: &source_parent,
                source_name,
            },
        )),
    }
}

struct CompensationContext<'a> {
    original_target_proof: &'a FilesystemIdentityProof,
    stable_target: &'a Path,
    destination_parent: &'a super::rename::PinnedDirectory,
    destination_name: &'a std::ffi::OsStr,
    stable_source: &'a Path,
    source_parent: &'a super::rename::PinnedDirectory,
    source_name: &'a std::ffi::OsStr,
}

fn classify_compensation_failure(
    binding_error: io::Error,
    restoration_error: TrackedMoveError,
    context: CompensationContext<'_>,
) -> TrackedMoveError {
    let TrackedMoveError {
        error: restoration_error,
        target_proof: restored_source_proof,
        target_path: restored_source_path,
    } = restoration_error;
    let combined = io::Error::other(format!(
        "{binding_error}; source restoration failed: {restoration_error}"
    ));
    if context
        .original_target_proof
        .validate(context.stable_target)
        .is_ok()
    {
        let actual_target = context
            .destination_parent
            .current_path()
            .map(|parent| parent.join(context.destination_name))
            .unwrap_or_else(|_| context.stable_target.to_path_buf());
        return TrackedMoveError::target_owned(
            combined,
            &actual_target,
            context.original_target_proof,
        );
    }
    if let Some(restored_source_proof) = restored_source_proof {
        if restored_source_proof
            .validate(context.stable_source)
            .is_ok()
        {
            let actual_source = context
                .source_parent
                .current_path()
                .map(|parent| parent.join(context.source_name))
                .unwrap_or_else(|_| context.stable_source.to_path_buf());
            return TrackedMoveError::target_owned(
                combined,
                &actual_source,
                &restored_source_proof,
            );
        }
        if let Some(restored_source_path) = restored_source_path {
            if restored_source_proof
                .validate(&restored_source_path)
                .is_ok()
            {
                return TrackedMoveError::target_owned(
                    combined,
                    &restored_source_path,
                    &restored_source_proof,
                );
            }
        }
    }
    TrackedMoveError::unchanged(combined)
}

fn rename_cross_drive_fallback_tracked_with(
    from: &Path,
    to: &Path,
    target_parent: &Path,
    publish: &impl Fn(&Path) -> io::Result<()>,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    rename_cross_drive_fallback_tracked_with_source(from, to, target_parent, None, &|source, _| {
        publish(source)
    })
}

fn rename_cross_drive_fallback_tracked_with_source(
    from: &Path,
    to: &Path,
    target_parent: &Path,
    owned_source: Option<OwnedDirectorySource<'_>>,
    publish: &impl Fn(&Path, Option<&super::rename::PinnedDirectory>) -> io::Result<()>,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    let source_proof = match owned_source.as_ref() {
        Some(source) => source.proof.clone(),
        None => FilesystemIdentityProof::capture(from)
            .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?,
    };
    let Err(error) = publish(from, owned_source.as_ref().map(|source| &source.directory)) else {
        source_proof
            .validate(to)
            .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
        return Ok(source_proof);
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

    fs::create_dir_all(target_parent).map_err(TrackedMoveError::unchanged)?;

    let publish_staging = |source: &Path| publish(source, None);
    if owned_source.is_some() || from.is_dir() {
        move_directory_across_volumes(
            from,
            to,
            target_parent,
            &source_proof,
            &publish_staging,
            owned_source,
        )
    } else {
        move_file_across_volumes(from, to, target_parent, &source_proof, &publish_staging)
    }
}

/// Convenience wrapper for callers that do not maintain a move journal.
pub fn rename_cross_drive_fallback(from: &Path, to: &Path) -> io::Result<()> {
    rename_cross_drive_fallback_tracked(from, to)
        .map(|_| ())
        .map_err(TrackedMoveError::into_io_error)
}

fn move_directory_across_volumes(
    from: &Path,
    to: &Path,
    target_parent: &Path,
    source_proof: &FilesystemIdentityProof,
    publish: &impl Fn(&Path) -> io::Result<()>,
    owned_source: Option<OwnedDirectorySource<'_>>,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    source_proof
        .validate(from)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
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

    let target_proof = FilesystemIdentityProof::capture(staging.path())
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    publish(staging.path()).map_err(TrackedMoveError::unchanged)?;
    let _published_staging_path = staging.keep();
    retire_source_or_remove_published_target(from, to, source_proof, &target_proof, owned_source)
}

fn move_file_across_volumes(
    from: &Path,
    to: &Path,
    target_parent: &Path,
    source_proof: &FilesystemIdentityProof,
    publish: &impl Fn(&Path) -> io::Result<()>,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    source_proof
        .validate(from)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    let mut staging =
        tempfile::NamedTempFile::new_in(target_parent).map_err(TrackedMoveError::unchanged)?;
    fs::copy(from, staging.path()).map_err(TrackedMoveError::unchanged)?;
    staging
        .as_file()
        .sync_all()
        .map_err(TrackedMoveError::unchanged)?;
    let target_proof = FilesystemIdentityProof::capture(staging.path())
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    publish(staging.path()).map_err(TrackedMoveError::unchanged)?;
    staging.disable_cleanup(true);
    retire_source_or_remove_published_target(from, to, source_proof, &target_proof, None)
}

fn retire_source_or_remove_published_target(
    from: &Path,
    to: &Path,
    source_proof: &FilesystemIdentityProof,
    target_proof: &FilesystemIdentityProof,
    owned_source: Option<OwnedDirectorySource<'_>>,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    target_proof
        .validate(to)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    let tombstone = source_tombstone_path(from);
    let retirement = source_proof
        .validate(from)
        .map_err(|error| io::Error::other(error.to_string()))
        .and_then(|()| match owned_source.as_ref() {
            Some(source) => source.directory.rename_into(
                source.parent,
                tombstone
                    .file_name()
                    .ok_or_else(|| io::Error::other("Source retirement has no filename"))?,
            ),
            None => super::rename::rename_no_replace(from, &tombstone),
        });
    drop(owned_source);
    if let Err(retire_error) = retirement {
        return match remove_owned_path(to, target_proof) {
            Ok(()) => Err(TrackedMoveError::unchanged(retire_error)),
            Err(cleanup_error) => {
                let error = io::Error::other(format!("Could not retire source after publishing destination: {retire_error}; owned destination cleanup requires repair: {cleanup_error}"));
                Err(if target_proof.validate(to).is_ok() {
                    TrackedMoveError::target_owned(error, to, target_proof)
                } else {
                    TrackedMoveError::unchanged(error)
                })
            }
        };
    }
    if let Err(error) = source_proof
        .validate(&tombstone)
        .and_then(|()| target_proof.validate(to))
    {
        let restore = super::rename::rename_no_replace(&tombstone, from);
        let error = io::Error::other(format!("Cross-volume retirement identity changed: {error}; source restoration: {}; inspect '{}' before retrying", restore.err().map_or_else(|| "restored".to_string(), |error| error.to_string()), tombstone.display()));
        return Err(if target_proof.validate(to).is_ok() {
            TrackedMoveError::target_owned(error, to, target_proof)
        } else {
            TrackedMoveError::unchanged(error)
        });
    }
    if let Err(error) = remove_owned_path(&tombstone, source_proof) {
        if error.kind() == io::ErrorKind::InvalidData {
            let error = io::Error::other(format!("Source cleanup identity changed; preserve recovery evidence and inspect '{}': {error}", tombstone.display()));
            return Err(if target_proof.validate(to).is_ok() {
                TrackedMoveError::target_owned(error, to, target_proof)
            } else {
                TrackedMoveError::unchanged(error)
            });
        }
        // The visible move is already complete and the destination contains a
        // full copy. Keep the recoverable hidden source residue instead of
        // reporting a failed move that a caller might retry into a collision.
        log::warn!(
            "Cross-volume move completed, but source tombstone '{}' could not be removed: {error}",
            tombstone.display()
        );
    }
    target_proof
        .validate(to)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    Ok(target_proof.clone())
}

pub(crate) fn remove_owned_path(path: &Path, proof: &FilesystemIdentityProof) -> io::Result<()> {
    proof
        .validate(path)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let quarantine = source_tombstone_path(path);
    super::rename::rename_no_replace(path, &quarantine)?;
    if let Err(error) = proof.validate(&quarantine) {
        let restore = super::rename::rename_no_replace(&quarantine, path);
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Cleanup identity changed: {error}; restoration: {}; preserved at '{}' if restoration failed", restore.err().map_or_else(|| "restored".to_string(), |error| error.to_string()), quarantine.display())));
    }
    if let Err(error) = remove_path(&quarantine) {
        let restore = super::rename::rename_no_replace(&quarantine, path);
        return Err(io::Error::other(format!(
            "Owned cleanup failed: {error}; restoration: {}; recoverable residue: '{}'",
            restore
                .err()
                .map_or_else(|| "restored".to_string(), |error| error.to_string()),
            quarantine.display()
        )));
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
    fn owned_import_fallback_rejects_replacement_after_source_validation() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("selected");
        let parked = temp.path().join("parked");
        let parent_path = temp.path().join("destination");
        let target = parent_path.join("imported");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&parent_path).unwrap();
        std::fs::write(source.join("asset.bin"), b"selected payload").unwrap();
        let expected = super::FilesystemIdentityProof::capture(&source).unwrap();
        let parent = super::super::rename::PinnedDirectory::open(&parent_path).unwrap();

        let error = super::rename_cross_drive_fallback_tracked_into_with_hook(
            &source,
            &target,
            &parent,
            Some(&expected),
            || {
                std::fs::rename(&source, &parked).unwrap();
                std::fs::create_dir(&source).unwrap();
                std::fs::write(source.join("asset.bin"), b"replacement payload").unwrap();
            },
            || {},
        )
        .unwrap_err();

        assert!(error.into_io_error().to_string().contains("changed"));
        assert!(!target.exists());
        assert_eq!(
            std::fs::read(source.join("asset.bin")).unwrap(),
            b"replacement payload"
        );
        assert_eq!(
            std::fs::read(parked.join("asset.bin")).unwrap(),
            b"selected payload"
        );
    }

    #[cfg(windows)]
    #[test]
    fn owned_import_copy_retains_source_handle_through_retirement() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("selected");
        let parked = temp.path().join("parked");
        let parent_path = temp.path().join("destination");
        let target = parent_path.join("imported");
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::create_dir(&parent_path).unwrap();
        std::fs::write(source.join("nested/asset.bin"), b"selected payload").unwrap();
        let expected = super::FilesystemIdentityProof::capture(&source).unwrap();
        let source_parent = super::super::rename::PinnedDirectory::open(temp.path()).unwrap();
        let destination_parent = super::super::rename::PinnedDirectory::open(&parent_path).unwrap();
        let owned_source = super::OwnedDirectorySource {
            directory: super::super::rename::PinnedDirectory::open_for_rename(&source).unwrap(),
            parent: &source_parent,
            proof: expected.clone(),
        };

        let target_proof = super::rename_cross_drive_fallback_tracked_with_source(
            &source,
            &target,
            &parent_path,
            Some(owned_source),
            &|staging, original| {
                std::fs::rename(&source, &parked)
                    .expect_err("copied import source must remain pinned until retirement");
                if original.is_some() {
                    Err(std::io::Error::from_raw_os_error(17))
                } else {
                    super::super::rename::rename_no_replace_into(
                        staging,
                        &destination_parent,
                        target.file_name().unwrap(),
                    )
                }
            },
        )
        .unwrap();

        assert_ne!(expected.identity(), target_proof.identity());
        target_proof.validate(&target).unwrap();
        assert_eq!(
            std::fs::read(target.join("nested/asset.bin")).unwrap(),
            b"selected payload"
        );
        assert!(!source.exists());
        assert!(!parked.exists());
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(&parent_path).unwrap().count(), 1);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn compensation_failure_retains_published_source_ownership() {
        let temp = tempfile::tempdir().unwrap();
        let source_parent_path = temp.path().join("source");
        let destination_parent_path = temp.path().join("destination");
        std::fs::create_dir(&source_parent_path).unwrap();
        std::fs::create_dir(&destination_parent_path).unwrap();
        let source_parent =
            super::super::rename::PinnedDirectory::open(&source_parent_path).unwrap();
        let destination_parent =
            super::super::rename::PinnedDirectory::open(&destination_parent_path).unwrap();
        let source_name = std::ffi::OsStr::new("restored");
        let destination_name = std::ffi::OsStr::new("published");
        let stable_source = source_parent.stable_path().join(source_name);
        let stable_target = destination_parent.stable_path().join(destination_name);
        std::fs::create_dir(&stable_target).unwrap();
        let original_target_proof =
            super::FilesystemIdentityProof::capture(&stable_target).unwrap();
        let retirement_residue = destination_parent.stable_path().join("retirement-residue");
        std::fs::rename(&stable_target, &retirement_residue).unwrap();
        std::fs::create_dir(&stable_source).unwrap();
        let restored_source_proof =
            super::FilesystemIdentityProof::capture(&stable_source).unwrap();
        let restoration_error = super::TrackedMoveError::target_owned(
            std::io::Error::other("injected cleanup failure after source publication"),
            &stable_source,
            &restored_source_proof,
        );

        let error = super::classify_compensation_failure(
            std::io::Error::other("destination namespace changed"),
            restoration_error,
            super::CompensationContext {
                original_target_proof: &original_target_proof,
                stable_target: &stable_target,
                destination_parent: &destination_parent,
                destination_name,
                stable_source: &stable_source,
                source_parent: &source_parent,
                source_name,
            },
        );

        assert!(error.target_is_owned());
        assert_eq!(
            error.target_path(),
            Some(source_parent_path.join(source_name).as_path())
        );
        error
            .target_proof()
            .unwrap()
            .validate(&source_parent_path.join(source_name))
            .unwrap();
        assert!(retirement_residue.exists());
    }

    #[cfg(windows)]
    #[test]
    fn namespace_proof_rejects_same_identity_junction_alias() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        let parent = root.join("Alice");
        let source = parent.join("Skin");
        std::fs::create_dir_all(&source).unwrap();
        let proof =
            super::FilesystemNamespaceProof::capture(&root, std::slice::from_ref(&source)).unwrap();
        let moved_parent = root.join("OriginalAlice");
        std::fs::rename(&parent, &moved_parent).unwrap();
        let output = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&parent)
            .arg(&moved_parent)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "junction fixture creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            super::filesystem_identity(&source),
            super::filesystem_identity(&moved_parent.join("Skin"))
        );
        assert!(
            proof.validate_paths(&[source]).is_err(),
            "same native ID through a new junction must not rebind ancestry"
        );
    }

    #[test]
    fn namespace_proof_keeps_prefix_aliases_but_rejects_replaced_ancestor_with_original_child() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        let source = root.join("Alice/Skin");
        std::fs::create_dir_all(&source).unwrap();
        let proof =
            super::FilesystemNamespaceProof::capture(&root, std::slice::from_ref(&source)).unwrap();
        let disabled = root.join("Alice/DISABLED Skin");
        std::fs::rename(&source, &disabled).unwrap();
        proof
            .validate_paths(std::slice::from_ref(&disabled))
            .unwrap();
        std::fs::rename(root.join("Alice"), root.join("OriginalAlice")).unwrap();
        std::fs::create_dir(root.join("Alice")).unwrap();
        std::fs::rename(root.join("OriginalAlice/DISABLED Skin"), &disabled).unwrap();
        assert!(proof.validate_paths(&[disabled]).is_err());
        assert!(proof
            .validate_paths(&[root.join("Uncaptured/Skin")])
            .is_err());
        assert!(
            super::FilesystemNamespaceProof::capture(&root, &[temp.path().join("Outside")])
                .is_err()
        );
    }

    #[test]
    fn copied_move_preserves_source_replaced_before_retirement() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("payload"), "original").unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("payload"), "original").unwrap();
        let source_proof = super::FilesystemIdentityProof::capture(&source).unwrap();
        let target_proof = super::FilesystemIdentityProof::capture(&target).unwrap();
        std::fs::rename(&source, root.path().join("original-source")).unwrap();
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("foreign"), "must survive").unwrap();

        assert!(super::retire_source_or_remove_published_target(
            &source,
            &target,
            &source_proof,
            &target_proof,
            None,
        )
        .is_err());
        assert_eq!(
            std::fs::read_to_string(source.join("foreign")).unwrap(),
            "must survive"
        );
    }

    #[test]
    fn copied_move_preserves_destination_replaced_before_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        let source_proof = super::FilesystemIdentityProof::capture(&source).unwrap();
        let target_proof = super::FilesystemIdentityProof::capture(&target).unwrap();
        std::fs::rename(&source, root.path().join("original-source")).unwrap();
        std::fs::rename(&target, root.path().join("published-target")).unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("foreign"), "must survive").unwrap();

        assert!(super::retire_source_or_remove_published_target(
            &source,
            &target,
            &source_proof,
            &target_proof,
            None,
        )
        .is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("foreign")).unwrap(),
            "must survive"
        );
    }

    #[test]
    fn owned_cleanup_rejects_a_foreign_replacement_without_removing_it() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let proof = super::FilesystemIdentityProof::capture(&target).unwrap();
        std::fs::rename(&target, root.path().join("owned-published-target")).unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("foreign"), "must survive").unwrap();
        let error = super::remove_owned_path(&target, &proof).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::read_to_string(target.join("foreign")).unwrap(),
            "must survive"
        );
    }

    #[test]
    fn identity_proof_rejects_replacement_and_missing_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("Skin");
        let moved = root.path().join("DISABLED Skin");
        std::fs::create_dir(&source).unwrap();
        let proof = super::FilesystemIdentityProof::capture(&source).unwrap();
        proof.validate(&source).unwrap();
        std::fs::rename(&source, &moved).unwrap();
        assert!(proof.validate(&source).is_err());
        proof.validate(&moved).unwrap();
        std::fs::create_dir(&source).unwrap();
        assert!(proof.validate(&source).is_err());
    }

    #[test]
    fn fallback_rename_does_not_replace_an_existing_file_or_empty_directory() {
        let root = tempfile::tempdir().unwrap();
        for is_directory in [false, true] {
            let (source, target) = if is_directory {
                let source = root.path().join("source-dir");
                let target = root.path().join("target-dir");
                std::fs::create_dir(&source).unwrap();
                std::fs::create_dir(&target).unwrap();
                (source, target)
            } else {
                let source = root.path().join("source.ini");
                let target = root.path().join("target.ini");
                std::fs::write(&source, "source").unwrap();
                std::fs::write(&target, "destination").unwrap();
                (source, target)
            };
            let identity = super::filesystem_identity(&target).unwrap();
            let error = super::rename_cross_drive_fallback_tracked(&source, &target)
                .expect_err("existing destination must never be replaced");
            assert!(!error.target_is_owned());
            assert!(source.exists());
            assert_eq!(
                super::filesystem_identity(&target).as_deref(),
                Some(identity.as_str())
            );
        }
    }

    #[test]
    fn directory_fallback_publishes_the_exact_requested_shape() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("archive-root");
        let target_parent = root.path().join("Ayaka");
        let target = target_parent.join("DISABLED Skin");
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::create_dir_all(&target_parent).unwrap();
        std::fs::write(source.join("nested/mod.ini"), "[TextureOverride]").unwrap();

        let source_proof = super::FilesystemIdentityProof::capture(&source).unwrap();
        let target_proof = move_directory_across_volumes(
            &source,
            &target,
            &target_parent,
            &source_proof,
            &|staging| super::super::rename::rename_no_replace(staging, &target),
            None,
        )
        .unwrap();
        target_proof.validate(&target).unwrap();
        assert_ne!(source_proof.identity(), target_proof.identity());

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

        let source_proof = super::FilesystemIdentityProof::capture(&source).unwrap();
        let target_proof = move_file_across_volumes(
            &source,
            &target,
            &target_parent,
            &source_proof,
            &|staging| super::super::rename::rename_no_replace(staging, &target),
        )
        .unwrap();
        target_proof.validate(&target).unwrap();
        assert_ne!(source_proof.identity(), target_proof.identity());

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "archive");
        assert!(!source.exists());
    }
}
