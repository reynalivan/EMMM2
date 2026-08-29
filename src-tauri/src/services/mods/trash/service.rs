//! Service-level soft delete. The command's trailing disk reconcile owns DB,
//! runtime projection, and collection missing-state updates.

use super::store::move_to_trash;
use super::types::DeleteModResult;
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::errors::AppError;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::scanner::watcher::WatcherState;
use std::path::Path;

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
