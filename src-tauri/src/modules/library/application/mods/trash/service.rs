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
}

impl PreparedTrashMove {
    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn quarantine(&self) -> &Path {
        &self.quarantine
    }

    pub fn execute(&self, state: &WatcherState) -> Result<(), AppError> {
        let _guard = state
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        std::fs::rename(&self.source, &self.quarantine)
            .map_err(|error| AppError::Io(error.to_string()))
    }

    pub fn rollback(&self, state: &WatcherState) -> Result<(), AppError> {
        let _guard = state
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        if self.source.exists() {
            return Ok(());
        }
        if !self.quarantine.exists() {
            return Err(AppError::Io(format!(
                "Trash rollback source is missing: {}",
                self.quarantine.display()
            )));
        }
        std::fs::rename(&self.quarantine, &self.source)
            .map_err(|error| AppError::Io(error.to_string()))
    }

    pub fn finalize(&self) -> Result<(), AppError> {
        if !self.quarantine.exists() {
            return Ok(());
        }
        move_to_trash(&self.quarantine)
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
    let quarantine = parent.join(format!(".emmm-trash-{}", uuid::Uuid::new_v4().simple()));
    Ok(PreparedTrashMove {
        source: source.to_path_buf(),
        quarantine,
    })
}

pub fn finalize_recorded_quarantines(
    quarantines: impl IntoIterator<Item = PathBuf>,
) -> Vec<String> {
    quarantines
        .into_iter()
        .filter(|path| path.exists())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".emmm-trash-"))
        })
        .filter_map(|path| {
            move_to_trash(&path)
                .err()
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect()
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
