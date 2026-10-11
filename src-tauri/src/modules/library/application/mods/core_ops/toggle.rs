//! Enable/disable a mod folder on disk. Callers converge the DB through Disk
//! Reconcile after releasing the operation lock.

use super::naming::{
    find_existing_destination_case_insensitive, rename_conflict_error, standardize_prefix,
    SiblingNameIndex,
};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::rename::PinnedDirectory;
use crate::platform::fs::retry::{blocking_error, RetryBudget};
use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToggleRenamePlan {
    old_path: PathBuf,
    new_path: PathBuf,
    expected_identity: String,
    expected_parent_identity: String,
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

    pub(crate) async fn apply_async(
        &self,
        noun: &str,
        budget: &mut RetryBudget,
    ) -> Result<(), AppError> {
        self.rename_async(false, noun, budget).await
    }

    pub(crate) async fn rollback_async(
        &self,
        noun: &str,
        budget: &mut RetryBudget,
    ) -> Result<(), AppError> {
        self.rename_async(true, noun, budget).await
    }

    async fn rename_async(
        &self,
        reverse: bool,
        noun: &str,
        budget: &mut RetryBudget,
    ) -> Result<(), AppError> {
        let result = budget
            .run(|| {
                let plan = self.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        let (source, destination) = if reverse {
                            (&plan.new_path, &plan.old_path)
                        } else {
                            (&plan.old_path, &plan.new_path)
                        };
                        plan.rename_checked_io(source, destination)
                    })
                    .await
                    .map_err(blocking_error)?
                }
            })
            .await;
        let (source, destination) = if reverse {
            (&self.new_path, &self.old_path)
        } else {
            (&self.old_path, &self.new_path)
        };
        result.map_err(|error| self.map_rename_failure(source, destination, noun, error))
    }

    fn rename_checked(
        &self,
        source: &Path,
        destination: &Path,
        noun: &str,
    ) -> Result<(), AppError> {
        self.rename_checked_io(source, destination)
            .map_err(|error| self.map_rename_failure(source, destination, noun, error))
    }

    fn rename_checked_io(&self, source: &Path, destination: &Path) -> std::io::Result<()> {
        self.rename_checked_io_with_hook(source, destination, || {})
    }

    fn rename_checked_io_with_hook(
        &self,
        source: &Path,
        destination: &Path,
        before_publication: impl FnOnce(),
    ) -> std::io::Result<()> {
        let parent = source
            .parent()
            .ok_or_else(|| std::io::Error::other("Invalid path"))?;
        if destination.parent() != Some(parent) {
            return Err(std::io::Error::other(
                "Toggle rename must remain in one parent",
            ));
        }
        if crate::platform::fs::file_utils::filesystem_identity(parent).as_deref()
            != Some(self.expected_parent_identity.as_str())
        {
            return Err(std::io::Error::other(
                "Toggle parent changed after planning",
            ));
        }
        let namespace_pins = self
            .namespace_proof
            .as_ref()
            .map(|proof| {
                proof
                    .pin_paths(&[source.to_path_buf(), destination.to_path_buf()])
                    .map_err(|error| std::io::Error::other(error.to_string()))
            })
            .transpose()?;
        let fallback_parent = if namespace_pins.is_none() {
            Some(PinnedDirectory::open(parent)?)
        } else {
            None
        };
        let parent_pin = if let Some(pins) = namespace_pins.as_ref() {
            let normalized_parent = crate::shared::path_key::physical_namespace_path(parent)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            pins.iter()
                .find(|pin| pin.path() == normalized_parent.as_path())
                .ok_or_else(|| std::io::Error::other("Toggle parent is not pinned"))?
        } else {
            fallback_parent
                .as_ref()
                .ok_or_else(|| std::io::Error::other("Toggle parent pin is unavailable"))?
        };
        if parent_pin.identity() != self.expected_parent_identity {
            return Err(std::io::Error::other("Toggle parent changed while pinning"));
        }
        let source_name = source
            .file_name()
            .ok_or_else(|| std::io::Error::other("Toggle source has no name"))?;
        let destination_name = destination
            .file_name()
            .ok_or_else(|| std::io::Error::other("Toggle destination has no name"))?;
        let owned_source =
            PinnedDirectory::open_for_rename(&parent_pin.stable_path().join(source_name))?;
        if owned_source.identity() != self.expected_identity {
            return Err(std::io::Error::other(format!(
                "Folder changed while preparing the rename: {}",
                source.display()
            )));
        }
        before_publication();
        owned_source.rename_into(parent_pin, destination_name)
    }

    fn map_rename_failure(
        &self,
        source: &Path,
        destination: &Path,
        noun: &str,
        error: std::io::Error,
    ) -> AppError {
        if let Some(proof) = &self.namespace_proof {
            if let Err(error) =
                proof.validate_paths(&[source.to_path_buf(), destination.to_path_buf()])
            {
                return error;
            }
        }
        if source
            .parent()
            .and_then(crate::platform::fs::file_utils::filesystem_identity)
            .as_deref()
            != Some(self.expected_parent_identity.as_str())
        {
            return AppError::Validation(
                "Toggle parent changed before lock diagnostics; refresh before retrying"
                    .to_string(),
            );
        }
        let destination_name = destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        if let Some(parent) = source.parent() {
            if let Some(existing_path) =
                find_existing_destination_case_insensitive(parent, &destination_name, source)
            {
                let source_name = source.file_name().unwrap_or_default().to_string_lossy();
                let base = crate::modules::workspace::domain::normalizer::normalize_display_name(
                    &source_name,
                );
                return rename_conflict_error(destination, &existing_path, &base);
            }
        }
        crate::platform::fs::retry::map_file_error_with_identity(
            source,
            &self.expected_identity,
            &format!("rename {noun}"),
            error,
        )
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
    let expected_parent_identity = crate::platform::fs::file_utils::filesystem_identity(parent)
        .ok_or_else(|| {
            AppError::Io(format!(
                "Could not establish toggle parent identity: {}",
                parent.display()
            ))
        })?;

    Ok(Some(ToggleRenamePlan {
        old_path: src.to_path_buf(),
        new_path,
        expected_identity,
        expected_parent_identity,
        namespace_proof: None,
    }))
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

    let Some(plan) = plan_toggle_rename(src, enable)? else {
        return Ok(path);
    };
    plan.apply_async("mod folder", &mut RetryBudget::default())
        .await?;
    let new_path = plan.new_path;

    log::info!(
        "Toggled mod: '{}' -> '{}'",
        src.file_name().unwrap_or_default().to_string_lossy(),
        new_path.display()
    );

    Ok(new_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::plan_toggle_rename;
    use crate::platform::fs::retry::map_file_error;

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

    #[cfg(windows)]
    #[test]
    fn checked_toggle_owns_source_through_publication() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("DISABLED Blue");
        let destination = temp.path().join("Blue");
        let parked = temp.path().join("original");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("original.txt"), b"A").unwrap();
        let original_id = crate::platform::fs::file_utils::filesystem_identity(&source).unwrap();
        let plan = plan_toggle_rename(&source, true).unwrap().unwrap();

        let mut competing_rename_blocked = false;
        let result = plan.rename_checked_io_with_hook(&source, &destination, || {
            competing_rename_blocked = std::fs::rename(&source, &parked).is_err();
            if !competing_rename_blocked {
                std::fs::create_dir(&source).unwrap();
                std::fs::write(source.join("replacement.txt"), b"B").unwrap();
            }
        });

        assert!(
            competing_rename_blocked,
            "owned source handle must block a competing rename"
        );
        result.expect("original A must publish successfully");
        assert_eq!(
            crate::platform::fs::file_utils::filesystem_identity(&destination).as_deref(),
            Some(original_id.as_str())
        );
        assert_eq!(
            std::fs::read(destination.join("original.txt")).unwrap(),
            b"A"
        );
        assert!(!source.exists());
        assert!(!parked.exists());
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
        let error = map_file_error(
            std::path::Path::new("C:\\Mods\\Blue"),
            "rename mod folder",
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
    #[tokio::test]
    async fn transient_directory_sharing_violation_retries_rename() {
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

        plan.apply_async(
            "mod folder",
            &mut crate::platform::fs::retry::RetryBudget::default(),
        )
        .await
        .expect("retry after handle release");
        release.join().expect("release thread");
        assert!(temp.path().join("Blue").is_dir());
    }

    #[cfg(windows)]
    fn delete_denying_directory_handle(path: &std::path::Path) -> std::fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .unwrap()
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn replacement_during_retry_delay_is_not_renamed() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("DISABLED Blue");
        let parked = temp.path().join("original");
        std::fs::create_dir(&source).unwrap();
        let plan = plan_toggle_rename(&source, true).unwrap().unwrap();
        let mut handle = Some(delete_denying_directory_handle(&source));
        let mut attempts = 0;
        let result = crate::platform::fs::retry::RetryBudget::default()
            .run(|| {
                attempts += 1;
                let result = plan.rename_checked_io(&source, &temp.path().join("Blue"));
                if attempts == 1 {
                    let error = result
                        .as_ref()
                        .expect_err("first attempt must hit the held handle");
                    assert!(crate::platform::fs::retry::is_file_lock(error));
                    drop(handle.take());
                    std::fs::rename(&source, &parked).unwrap();
                    std::fs::create_dir(&source).unwrap();
                    std::fs::write(source.join("replacement.txt"), "untouched").unwrap();
                }
                std::future::ready(result)
            })
            .await;
        assert_eq!(attempts, 2);
        assert!(result.unwrap_err().to_string().contains("Folder changed"));
        assert_eq!(
            std::fs::read_to_string(source.join("replacement.txt")).unwrap(),
            "untouched"
        );
        assert!(!temp.path().join("Blue").exists());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn destination_created_during_retry_delay_is_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("DISABLED Blue");
        let destination = temp.path().join("Blue");
        std::fs::create_dir(&source).unwrap();
        let plan = plan_toggle_rename(&source, true).unwrap().unwrap();
        let mut handle = Some(delete_denying_directory_handle(&source));
        let mut attempts = 0;
        let result = crate::platform::fs::retry::RetryBudget::default()
            .run(|| {
                attempts += 1;
                let result = plan.rename_checked_io(&source, &destination);
                if attempts == 1 {
                    let error = result
                        .as_ref()
                        .expect_err("first attempt must hit the held handle");
                    assert!(crate::platform::fs::retry::is_file_lock(error));
                    std::fs::create_dir(&destination).unwrap();
                    std::fs::write(destination.join("external.txt"), "untouched").unwrap();
                    drop(handle.take());
                }
                std::future::ready(result)
            })
            .await;
        assert_eq!(attempts, 2);
        let error =
            plan.map_rename_failure(&source, &destination, "mod folder", result.unwrap_err());
        assert!(error.to_string().contains("RenameConflict"));
        assert!(source.is_dir());
        assert_eq!(
            std::fs::read_to_string(destination.join("external.txt")).unwrap(),
            "untouched"
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_nested_file_lock_retains_the_actual_windows_error() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("DISABLED Blue");
        std::fs::create_dir(&source).unwrap();
        let file = source.join("mod.ini");
        std::fs::write(&file, "[Test]").unwrap();
        let _handle = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(&file)
            .unwrap();
        let result =
            crate::platform::fs::rename::rename_no_replace(&source, &temp.path().join("Blue"));
        if let Err(error) = result {
            assert!(matches!(error.raw_os_error(), Some(5 | 32 | 33)), "{error}");
            let code = error.raw_os_error();
            let mapped = map_file_error(&source, "rename mod folder", error);
            if code == Some(5) {
                assert!(matches!(mapped, crate::shared::errors::AppError::Io(_)));
            }
        }
    }
}
