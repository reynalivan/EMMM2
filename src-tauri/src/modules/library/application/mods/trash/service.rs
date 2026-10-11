//! Service-level soft delete. The command's trailing disk reconcile owns DB,
//! runtime projection, and collection missing-state updates.

use super::store::move_to_trash;
use super::types::DeleteModResult;
use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::ValidatedPath;
use crate::shared::errors::AppError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PreparedTrashMove {
    source: PathBuf,
    quarantine: PathBuf,
    expected_identity: String,
    expected_parent_identity: String,
}

impl PreparedTrashMove {
    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn quarantine(&self) -> &Path {
        &self.quarantine
    }

    pub fn expected_identity(&self) -> &str {
        &self.expected_identity
    }

    pub fn execute(&self, state: &WatcherState) -> Result<(), AppError> {
        let _guard = state
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        self.execute_disk().map_err(|error| {
            crate::platform::fs::retry::map_file_error_with_identity(
                &self.source,
                &self.expected_identity,
                "quarantine folder",
                error,
            )
        })
    }

    fn validate_parent(&self) -> std::io::Result<()> {
        let actual = self
            .source
            .parent()
            .and_then(crate::platform::fs::file_utils::filesystem_identity);
        if actual.as_deref() != Some(self.expected_parent_identity.as_str()) {
            return Err(std::io::Error::other(
                "Trash parent changed after validation",
            ));
        }
        Ok(())
    }

    fn execute_disk(&self) -> std::io::Result<()> {
        self.validate_parent()?;
        self.require_expected_identity(&self.source, "trash source")
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        if self.quarantine.exists() {
            return Err(std::io::Error::other(format!(
                "Trash quarantine destination already exists: {}",
                self.quarantine.display()
            )));
        }
        crate::platform::fs::rename::rename_no_replace(&self.source, &self.quarantine)
    }

    pub(crate) async fn execute_async(
        &self,
        state: &WatcherState,
        budget: &mut crate::platform::fs::retry::RetryBudget,
    ) -> Result<(), AppError> {
        let _guard = state
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        budget
            .run(|| {
                let plan = self.clone();
                async move {
                    tokio::task::spawn_blocking(move || plan.execute_disk())
                        .await
                        .map_err(crate::platform::fs::retry::blocking_error)?
                }
            })
            .await
            .map_err(|error| {
                crate::platform::fs::retry::map_file_error_with_identity(
                    &self.source,
                    &self.expected_identity,
                    "quarantine folder",
                    error,
                )
            })
    }

    pub async fn execute_with_retry(&self, state: &WatcherState) -> Result<(), AppError> {
        self.execute_async(
            state,
            &mut crate::platform::fs::retry::RetryBudget::default(),
        )
        .await
    }

    pub fn rollback(&self, state: &WatcherState) -> Result<(), AppError> {
        let _guard = state
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        self.rollback_disk().map_err(|error| {
            crate::platform::fs::retry::map_file_error_with_identity(
                &self.quarantine,
                &self.expected_identity,
                "restore folder",
                error,
            )
        })
    }

    fn rollback_disk(&self) -> std::io::Result<()> {
        self.validate_parent()?;
        let source_exists = self.source.exists();
        let quarantine_exists = self.quarantine.exists();
        if source_exists && quarantine_exists {
            return Err(std::io::Error::other(format!(
                "Trash rollback found both source and quarantine paths: {} and {}",
                self.source.display(),
                self.quarantine.display()
            )));
        }
        if source_exists {
            self.require_expected_identity(&self.source, "restored trash source")
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            return Ok(());
        }
        if !quarantine_exists {
            return Err(std::io::Error::other(format!(
                "Trash rollback source is missing: {}",
                self.quarantine.display()
            )));
        }
        self.require_expected_identity(&self.quarantine, "trash quarantine")
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        crate::platform::fs::rename::rename_no_replace(&self.quarantine, &self.source)
    }

    pub(crate) async fn rollback_async(
        &self,
        state: &WatcherState,
        budget: &mut crate::platform::fs::retry::RetryBudget,
    ) -> Result<(), AppError> {
        let _guard = state
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        budget
            .run(|| {
                let plan = self.clone();
                async move {
                    tokio::task::spawn_blocking(move || plan.rollback_disk())
                        .await
                        .map_err(crate::platform::fs::retry::blocking_error)?
                }
            })
            .await
            .map_err(|error| {
                crate::platform::fs::retry::map_file_error_with_identity(
                    &self.quarantine,
                    &self.expected_identity,
                    "restore folder",
                    error,
                )
            })
    }

    pub async fn rollback_with_retry(&self, state: &WatcherState) -> Result<(), AppError> {
        self.rollback_async(
            state,
            &mut crate::platform::fs::retry::RetryBudget::default(),
        )
        .await
    }

    pub fn finalize(&self) -> Result<(), AppError> {
        if !self.quarantine.exists() {
            return Ok(());
        }
        self.require_expected_identity(&self.quarantine, "trash quarantine")?;
        move_to_trash(&self.quarantine)
    }

    fn require_expected_identity(&self, path: &Path, label: &str) -> Result<(), AppError> {
        let actual = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(path);
        if actual.as_deref() == Some(self.expected_identity.as_str()) {
            Ok(())
        } else {
            Err(AppError::Io(format!(
                "{label} changed after validation: {}",
                path.display()
            )))
        }
    }
}

pub fn prepare_trash_move(source: &Path) -> Result<PreparedTrashMove, AppError> {
    if !source.exists() {
        return Err(AppError::Io(format!(
            "Source does not exist: {}",
            source.display()
        )));
    }
    if !source.is_dir() {
        return Err(AppError::Io("Only directories can be trashed".to_string()));
    }
    let parent = source
        .parent()
        .ok_or_else(|| AppError::Validation("Trash source has no parent directory".to_string()))?;
    let expected_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(source)
        .ok_or_else(|| {
            AppError::Io(format!(
                "Could not determine trash source identity: {}",
                source.display()
            ))
        })?;
    let quarantine = parent.join(format!(".emmm-trash-{}", uuid::Uuid::new_v4().simple()));
    let expected_parent_identity = crate::platform::fs::file_utils::filesystem_identity(parent)
        .ok_or_else(|| AppError::Io("Could not determine trash parent identity".to_string()))?;
    Ok(PreparedTrashMove {
        source: source.to_path_buf(),
        quarantine,
        expected_identity,
        expected_parent_identity,
    })
}

#[derive(Debug, Clone)]
pub struct RecordedQuarantine {
    pub path: PathBuf,
    pub source_path: Option<PathBuf>,
    pub game_root: PathBuf,
    pub expected_root_identity: Option<String>,
    pub expected_identity: Option<String>,
}

pub fn finalize_recorded_quarantines(
    quarantines: impl IntoIterator<Item = RecordedQuarantine>,
) -> Vec<String> {
    finalize_recorded_quarantines_with(quarantines, &move_to_trash)
}

pub(super) fn finalize_recorded_quarantines_with(
    quarantines: impl IntoIterator<Item = RecordedQuarantine>,
    recycle: &impl Fn(&Path) -> Result<(), AppError>,
) -> Vec<String> {
    quarantines
        .into_iter()
        .filter_map(|record| {
            let result = (|| {
                match std::fs::symlink_metadata(&record.path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(error) => return Err(error.into()),
                    Ok(_) => {}
                }
                let pins = validate_recorded_quarantine(&record)?;
                let parent = pins.last().ok_or_else(|| AppError::Validation("Trash parent proof is missing".to_string()))?;
                let target = crate::platform::fs::rename::PinnedDirectory::open_for_rename(&record.path)?;
                if Some(target.identity()) != record.expected_identity.as_deref() {
                    return Err(AppError::Validation("Trash target changed before cleanup".to_string()));
                }
                let cleanup_name = std::ffi::OsString::from(format!(".emmm-trash-cleanup-{}", uuid::Uuid::new_v4().simple()));
                let cleanup = parent.stable_path().join(&cleanup_name);
                target.rename_into(parent, &cleanup_name)?;
                drop(target);
                let cleanup_matches = crate::platform::fs::file_utils::filesystem_identity(&cleanup).as_deref() == record.expected_identity.as_deref();
                let result = if cleanup_matches { recycle(&cleanup) } else { Err(AppError::Validation("Trash target changed during cleanup handoff".to_string())) };
                if let Err(error) = result {
                    let restore = crate::platform::fs::rename::rename_no_replace_into(&cleanup, parent, record.path.file_name().ok_or_else(|| AppError::Validation("Trash filename is missing".to_string()))?);
                    return Err(match restore {
                        Ok(()) => error,
                        Err(restore_error) => AppError::Io(format!("Trash cleanup failed ({error}); restore failed ({restore_error}); retained at {}", parent.current_path()?.join(&cleanup_name).display())),
                    });
                }
                Ok(())
            })();
            result.err().map(|error| format!("{}: {error}", record.path.display()))
        })
        .collect()
}

fn validate_recorded_quarantine(
    record: &RecordedQuarantine,
) -> Result<Vec<crate::platform::fs::rename::PinnedDirectory>, AppError> {
    let invalid = || {
        AppError::Validation(
            "Trash cleanup ownership is unproven; preserving quarantine for manual recovery"
                .to_string(),
        )
    };
    let root_identity = record
        .expected_root_identity
        .as_deref()
        .ok_or_else(invalid)?;
    let target_identity = record.expected_identity.as_deref().ok_or_else(invalid)?;
    let source = record.source_path.as_deref().ok_or_else(invalid)?;
    if !record.path.is_absolute()
        || !record.game_root.is_absolute()
        || record
            .path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || source.parent() != record.path.parent()
        || record
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(".emmm-trash-"))
            .is_none_or(|id| uuid::Uuid::parse_str(id).is_err())
    {
        return Err(invalid());
    }
    let aligned_path = crate::shared::path_key::physical_namespace_path(&record.path)?;
    let aligned_root = crate::shared::path_key::physical_namespace_path(&record.game_root)?;
    let relative = aligned_path
        .strip_prefix(&aligned_root)
        .map_err(|_| invalid())?;
    let mut directory = record.game_root.clone();
    let mut pins = Vec::new();
    let mut retain = |path: &Path| -> Result<(), AppError> {
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(invalid());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes()
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                != 0
            {
                return Err(invalid());
            }
        }
        pins.push(crate::platform::fs::rename::PinnedDirectory::open(path)?);
        Ok(())
    };
    retain(&directory)?;
    for component in relative.parent().into_iter().flat_map(Path::components) {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(invalid());
        }
        directory.push(component.as_os_str());
        retain(&directory)?;
    }
    if pins.first().map(|root| root.identity()) != Some(root_identity) {
        return Err(invalid());
    }
    let metadata = std::fs::symlink_metadata(&record.path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
        {
            return Err(invalid());
        }
    }
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || crate::platform::fs::file_utils::filesystem_identity(&record.path).as_deref()
            != Some(target_identity)
    {
        return Err(invalid());
    }
    Ok(pins)
}

/// Move a mod folder to the OS recycle bin.
pub async fn delete_mod_service(
    state: &WatcherState,
    path: &ValidatedPath,
) -> Result<DeleteModResult, AppError> {
    let original = path.original();

    let _guard = state.suppressor.suppress_paths([original]);

    move_to_trash(Path::new(original))?;
    Ok(DeleteModResult {
        collection_impact: CollectionReferenceImpact::default(),
        sync_warning: None,
    })
}

/// Helper that suppresses the watcher for the single move action.
pub async fn move_to_trash_guarded(state: &WatcherState, path: String) -> Result<(), AppError> {
    let path_obj = Path::new(&path);
    let _guard = state.suppressor.suppress_paths([path_obj]);
    move_to_trash(path_obj)
}
