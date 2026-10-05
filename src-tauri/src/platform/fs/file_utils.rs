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
}

impl TrackedMoveError {
    fn unchanged(error: io::Error) -> Self {
        Self {
            error,
            target_proof: None,
        }
    }

    fn target_owned(error: io::Error, proof: &FilesystemIdentityProof) -> Self {
        Self {
            error,
            target_proof: Some(proof.clone()),
        }
    }

    pub fn target_is_owned(&self) -> bool {
        self.target_proof.is_some()
    }

    pub(crate) fn target_proof(&self) -> Option<&FilesystemIdentityProof> {
        self.target_proof.as_ref()
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
    let source_proof = FilesystemIdentityProof::capture(from)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    let Err(error) = super::rename::rename_no_replace(from, to) else {
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

    let parent = to.parent().ok_or_else(|| {
        TrackedMoveError::unchanged(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent",
        ))
    })?;
    fs::create_dir_all(parent).map_err(TrackedMoveError::unchanged)?;

    if from.is_dir() {
        move_directory_across_volumes(from, to, parent, &source_proof)
    } else {
        move_file_across_volumes(from, to, parent, &source_proof)
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
    super::rename::rename_no_replace(staging.path(), to).map_err(TrackedMoveError::unchanged)?;
    let _published_staging_path = staging.keep();
    retire_source_or_remove_published_target(from, to, source_proof, &target_proof)
}

fn move_file_across_volumes(
    from: &Path,
    to: &Path,
    target_parent: &Path,
    source_proof: &FilesystemIdentityProof,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    source_proof
        .validate(from)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    let staging =
        tempfile::NamedTempFile::new_in(target_parent).map_err(TrackedMoveError::unchanged)?;
    fs::copy(from, staging.path()).map_err(TrackedMoveError::unchanged)?;
    staging
        .as_file()
        .sync_all()
        .map_err(TrackedMoveError::unchanged)?;
    let target_proof = FilesystemIdentityProof::capture(staging.path())
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    staging
        .persist_noclobber(to)
        .map_err(|error| TrackedMoveError::unchanged(error.error))?;
    retire_source_or_remove_published_target(from, to, source_proof, &target_proof)
}

fn retire_source_or_remove_published_target(
    from: &Path,
    to: &Path,
    source_proof: &FilesystemIdentityProof,
    target_proof: &FilesystemIdentityProof,
) -> Result<FilesystemIdentityProof, TrackedMoveError> {
    target_proof
        .validate(to)
        .map_err(|error| TrackedMoveError::unchanged(io::Error::other(error.to_string())))?;
    let tombstone = source_tombstone_path(from);
    let retirement = source_proof
        .validate(from)
        .map_err(|error| io::Error::other(error.to_string()))
        .and_then(|()| super::rename::rename_no_replace(from, &tombstone));
    if let Err(retire_error) = retirement {
        return match remove_owned_path(to, target_proof) {
            Ok(()) => Err(TrackedMoveError::unchanged(retire_error)),
            Err(cleanup_error) => {
                let error = io::Error::other(format!("Could not retire source after publishing destination: {retire_error}; owned destination cleanup requires repair: {cleanup_error}"));
                Err(if target_proof.validate(to).is_ok() {
                    TrackedMoveError::target_owned(error, target_proof)
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
            TrackedMoveError::target_owned(error, target_proof)
        } else {
            TrackedMoveError::unchanged(error)
        });
    }
    if let Err(error) = remove_owned_path(&tombstone, source_proof) {
        if error.kind() == io::ErrorKind::InvalidData {
            let error = io::Error::other(format!("Source cleanup identity changed; preserve recovery evidence and inspect '{}': {error}", tombstone.display()));
            return Err(if target_proof.validate(to).is_ok() {
                TrackedMoveError::target_owned(error, target_proof)
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
            &target_proof
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
            &target_proof
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
        let target_proof =
            move_directory_across_volumes(&source, &target, &target_parent, &source_proof).unwrap();
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
        let target_proof =
            move_file_across_volumes(&source, &target, &target_parent, &source_proof).unwrap();
        target_proof.validate(&target).unwrap();
        assert_ne!(source_proof.identity(), target_proof.identity());

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "archive");
        assert!(!source.exists());
    }
}
