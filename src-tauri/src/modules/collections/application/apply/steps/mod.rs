//! Validation, planning, and rename stages for collection apply.

use crate::shared::errors::{AppError, CollectionError};

pub mod batch_rename;
pub mod resolve_current_state;
pub mod resolve_target;
pub mod validate_collection;
pub mod validate_paths;

mod rollback;

mod targets;
fn object_toggle_error(error: AppError) -> CollectionError {
    match error {
        AppError::FileInUse { path, processes } => CollectionError::FileInUse { path, processes },
        AppError::PathBusy { path } => CollectionError::PathBusy { path },
        other => CollectionError::Io(other.to_string()),
    }
}
