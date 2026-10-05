//! Enable/disable a mod folder on disk. Callers converge the DB through Disk
//! Reconcile after releasing the operation lock.

use super::naming::{
    find_existing_destination_case_insensitive, rename_conflict_error, standardize_prefix,
    SiblingNameIndex,
};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::rename::rename_no_replace;
use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};
use std::time::Duration;

const RENAME_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(15),
    Duration::from_millis(40),
    Duration::from_millis(80),
];

fn is_transient_rename_lock(error: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(32 | 33))
    }
    #[cfg(not(windows))]
    {
        let _ = error;
        false
    }
}

#[derive(Debug, Clone)]
pub struct ToggleRenamePlan {
    old_path: PathBuf,
    new_path: PathBuf,
    expected_identity: String,
    namespace_proof:
        Option<std::sync::Arc<crate::platform::fs::file_utils::FilesystemNamespaceProof>>,
}

impl ToggleRenamePlan {
    pub(crate) fn set_namespace_proof(
        &mut self,
        proof: std::sync::Arc<crate::platform::fs::file_utils::FilesystemNamespaceProof>,
    ) {
        self.namespace_proof = Some(proof);
    }
    pub fn old_path(&self) -> &Path {
        &self.old_path
    }

    pub fn new_path(&self) -> &Path {
        &self.new_path
    }

    pub fn expected_identity(&self) -> &str {
        &self.expected_identity
    }

    pub fn apply(&self, noun: &str) -> Result<(), AppError> {
        self.rename_checked(&self.old_path, &self.new_path, noun)
    }

    pub fn rollback(&self, noun: &str) -> Result<(), AppError> {
        self.rename_checked(&self.new_path, &self.old_path, noun)
    }

    fn rename_checked(
        &self,
        source: &Path,
        destination: &Path,
        noun: &str,
    ) -> Result<(), AppError> {
        let parent = source
            .parent()
            .ok_or_else(|| AppError::Io("Invalid path".to_string()))?;
        if destination.parent() != Some(parent) {
            return Err(AppError::Validation(
                "Toggle rename must remain in one parent".to_string(),
            ));
        }
        for retry_delay in RENAME_RETRY_DELAYS
            .iter()
            .map(Some)
            .chain(std::iter::once(None))
        {
            if let Some(proof) = &self.namespace_proof {
                proof.validate_paths(&[source.to_path_buf(), destination.to_path_buf()])?;
            }
            let actual_identity = crate::platform::fs::file_utils::filesystem_identity(source);
            if actual_identity.as_deref() != Some(self.expected_identity.as_str()) {
                return Err(AppError::Io(format!(
                    "Folder changed while preparing the rename: {}",
                    source.display()
                )));
            }
            let destination_name = destination
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            // Planning is a snapshot; atomic no-overwrite is the final namespace guard.
            match rename_no_replace(source, destination) {
                Ok(()) => return Ok(()),
                Err(error) if is_transient_rename_lock(&error) && retry_delay.is_some() => {
                    std::thread::sleep(*retry_delay.expect("retry guard checked above"));
                }
                Err(error) => {
                    if let Some(existing_path) = find_existing_destination_case_insensitive(
                        parent,
                        &destination_name,
                        source,
                    ) {
                        let source_name = source.file_name().unwrap_or_default().to_string_lossy();
                        let base =
                            crate::modules::workspace::domain::normalizer::normalize_display_name(
                                &source_name,
                            );
                        return Err(rename_conflict_error(destination, &existing_path, &base));
                    }
                    return Err(map_toggle_error(source, noun, error));
                }
            }
        }
        unreachable!("bounded rename attempts always return")
    }

    pub fn rebase_paths(&mut self, rewrites: &[(PathBuf, PathBuf)]) {
        self.old_path = rebase_path(self.old_path.clone(), rewrites);
        self.new_path = rebase_path(self.new_path.clone(), rewrites);
    }
}

fn rebase_path(mut path: PathBuf, rewrites: &[(PathBuf, PathBuf)]) -> PathBuf {
    for (old_path, new_path) in rewrites {
        if let Ok(suffix) = path.strip_prefix(old_path) {
            path = new_path.join(suffix);
        }
    }
    path
}

pub fn plan_toggle_rename(src: &Path, enable: bool) -> Result<Option<ToggleRenamePlan>, AppError> {
    plan_toggle_rename_with_sibling_index(src, enable, None)
}

pub fn plan_toggle_rename_with_sibling_index(
    src: &Path,
    enable: bool,
    sibling_index: Option<&SiblingNameIndex>,
) -> Result<Option<ToggleRenamePlan>, AppError> {
    if !src.exists() || !src.is_dir() {
        return Err(AppError::Io(format!(
            "Mod folder does not exist: {}",
            src.display()
        )));
    }
    let old_name = src.file_name().unwrap_or_default().to_string_lossy();
    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        return Ok(None);
    }

    let parent = src
        .parent()
        .ok_or_else(|| AppError::Io("Invalid path".to_string()))?;
    let new_path = parent.join(&new_name);
    let existing_path = match sibling_index {
        Some(index) => index.find_destination_collision(&new_name, src),
        None => find_existing_destination_case_insensitive(parent, &new_name, src),
    };
    if let Some(existing_path) = existing_path {
        let base = crate::modules::workspace::domain::normalizer::normalize_display_name(&old_name);
        return Err(rename_conflict_error(&new_path, &existing_path, &base));
    }

    let expected_identity =
        crate::platform::fs::file_utils::filesystem_identity(src).ok_or_else(|| {
            AppError::Io(format!(
                "Could not establish filesystem identity for {}",
                src.display()
            ))
        })?;

    Ok(Some(ToggleRenamePlan {
        old_path: src.to_path_buf(),
        new_path,
        expected_identity,
        namespace_proof: None,
    }))
}

/// Map a rename failure to a structured error, surfacing the locking
/// processes when the folder is busy.
pub(crate) fn map_toggle_error(src: &Path, noun: &str, error: std::io::Error) -> AppError {
    if is_transient_rename_lock(&error) {
        let processes = crate::platform::fs::locking::get_locking_processes(src);
        if !processes.is_empty() {
            return AppError::FileInUse {
                path: src.to_string_lossy().to_string(),
                processes,
            };
        }

        return AppError::PathBusy {
            path: src.to_string_lossy().to_string(),
        };
    }

    AppError::Io(format!(
        "Failed to rename {noun}: {error} (OS error {:?})",
        error.raw_os_error()
    ))
}

/// Rename `src` to its enabled/disabled form on disk.
/// Returns `Ok(None)` when the folder already has the desired prefix state.
pub(crate) fn rename_toggle_on_disk(
    src: &Path,
    enable: bool,
    noun: &str,
) -> Result<Option<PathBuf>, AppError> {
    let Some(plan) = plan_toggle_rename(src, enable)? else {
        return Ok(None);
    };
    plan.apply(noun)?;
    Ok(Some(plan.new_path))
}

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, AppError> {
    // Path-scoped: covers both spellings of the rename (same identity key)
    // and keeps suppressing through the async event tail after return.
    let _guard = state.suppressor.suppress_paths([&path]);

    let src = Path::new(&path);
    if !src.exists() || !src.is_dir() {
        return Err(AppError::Io(format!("Mod folder does not exist: {path}")));
    }

    let Some(new_path) = rename_toggle_on_disk(src, enable, "mod folder")? else {
        return Ok(path);
    };

    log::info!(
        "Toggled mod: '{}' -> '{}'",
        src.file_name().unwrap_or_default().to_string_lossy(),
        new_path.display()
    );

    Ok(new_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::{map_toggle_error, plan_toggle_rename};

    #[test]
    fn proven_toggle_rechecks_parent_ownership_at_storage_attempt() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        let source = root.join("Alice/Skin");
        std::fs::create_dir_all(&source).unwrap();
        let mut plan = plan_toggle_rename(&source, false).unwrap().unwrap();
        plan.set_namespace_proof(std::sync::Arc::new(
            crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
                &root,
                std::slice::from_ref(&source),
            )
            .unwrap(),
        ));
        std::fs::rename(root.join("Alice"), root.join("OriginalAlice")).unwrap();
        std::fs::create_dir(root.join("Alice")).unwrap();
        std::fs::rename(root.join("OriginalAlice/Skin"), &source).unwrap();
        assert!(plan.apply("mod folder").is_err());
        assert!(source.is_dir());
        assert!(!root.join("Alice/DISABLED Skin").exists());
    }

    #[test]
    fn prepared_toggle_rejects_a_replacement_source_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("DISABLED Blue");
        let parked = temp.path().join("parked");
        std::fs::create_dir(&source).expect("source");
        let plan = plan_toggle_rename(&source, true)
            .expect("plan")
            .expect("rename plan");

        std::fs::rename(&source, &parked).expect("park original");
        std::fs::create_dir(&source).expect("replacement source");

        let error = plan
            .apply("mod folder")
            .expect_err("replacement must be rejected");
        assert!(error
            .to_string()
            .contains("Folder changed while preparing the rename"));
        assert!(source.exists());
        assert!(!temp.path().join("Blue").exists());
    }

    #[test]
    fn prepared_toggle_rechecks_an_external_destination_collision() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("DISABLED Blue");
        let target = temp.path().join("Blue");
        std::fs::create_dir(&source).expect("source");
        let plan = plan_toggle_rename(&source, true)
            .expect("plan")
            .expect("rename required");

        std::fs::create_dir(&target).expect("external destination");

        let error = plan
            .apply("mod folder")
            .expect_err("late collision must be rejected");
        assert!(error.to_string().contains("RenameConflict"));
        assert!(source.exists());
        assert!(target.exists());
    }

    #[test]
    fn activation_conflicts_only_with_a_same_parent_folder_name() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first_parent = temp.path().join("Alice");
        let second_parent = temp.path().join("Bob");
        std::fs::create_dir_all(first_parent.join("DISABLED Blue")).expect("first mod");
        std::fs::create_dir_all(second_parent.join("Blue")).expect("other object mod");

        let first = plan_toggle_rename(&first_parent.join("DISABLED Blue"), true)
            .expect("same name under another parent is allowed")
            .expect("rename required");
        first.apply("mod folder").expect("first enable");
        assert!(first_parent.join("Blue").is_dir());
        assert!(second_parent.join("Blue").is_dir());

        std::fs::create_dir(first_parent.join("DISABLED Blue")).expect("colliding mod");
        let error = plan_toggle_rename(&first_parent.join("DISABLED Blue"), true)
            .expect_err("same parent destination must conflict");
        assert!(error.to_string().contains("RenameConflict"));
    }

    #[test]
    fn rollback_rejects_a_replacement_destination_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("DISABLED Blue");
        let target = temp.path().join("Blue");
        let parked = temp.path().join("parked");
        std::fs::create_dir(&source).expect("source");
        let plan = plan_toggle_rename(&source, true)
            .expect("plan")
            .expect("rename required");
        plan.apply("mod folder").expect("apply");
        std::fs::rename(&target, &parked).expect("park original");
        std::fs::create_dir(&target).expect("replacement destination");

        let error = plan.rollback("mod folder").expect_err("identity changed");
        assert!(error.to_string().contains("Folder changed"));
        assert!(target.exists());
        assert!(!source.exists());
        assert!(parked.exists());
    }

    #[cfg(windows)]
    #[test]
    fn access_denied_is_not_reported_as_path_busy() {
        let error = map_toggle_error(
            std::path::Path::new("C:\\Mods\\Blue"),
            "mod folder",
            std::io::Error::from_raw_os_error(5),
        );
        assert!(!matches!(
            error,
            crate::shared::errors::AppError::PathBusy { .. }
        ));
    }

    #[cfg(windows)]
    #[test]
    fn case_alias_of_source_is_not_a_collision() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("DISABLED Blue");
        std::fs::create_dir(&source).expect("source");
        let alias = temp.path().join("disabled blue");

        let plan = plan_toggle_rename(&alias, true)
            .expect("case alias must not conflict with itself")
            .expect("rename required");
        plan.apply("mod folder").expect("apply case-alias rename");
        assert!(temp.path().join("blue").is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn transient_directory_sharing_violation_retries_rename() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("DISABLED Blue");
        std::fs::create_dir(&source).expect("source");
        let plan = plan_toggle_rename(&source, true)
            .expect("plan")
            .expect("rename required");
        let blocking_handle = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(&source)
            .expect("open directory without delete sharing");
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            drop(blocking_handle);
        });

        plan.apply("mod folder")
            .expect("retry after handle release");
        release.join().expect("release thread");
        assert!(temp.path().join("Blue").is_dir());
    }
}
