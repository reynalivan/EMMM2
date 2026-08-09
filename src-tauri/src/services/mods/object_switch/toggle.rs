//! Explicit enable/disable of an object root folder (Workspace Switch).

use super::resolve::resolve_object_root_path;
use crate::domain::errors::AppError;
use crate::services::mods::core_ops::rename_toggle_on_disk;
use crate::services::scanner::watcher::WatcherState;
use std::path::Path;

pub struct ObjectSwitchOutcome {
    pub object_id: String,
    pub original_path: String,
    pub next_path: String,
}

/// Workspace Switch owns explicit object-root enable/disable.
/// Do not route object targets through mod-toggle services. Disk-only: the
/// caller's scoped reconcile settles the DB afterwards.
pub async fn toggle_object_root_service(
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    game_id: &str,
    object_id: &str,
    enable: bool,
) -> Result<ObjectSwitchOutcome, AppError> {
    let (object, mods_path, current_absolute_path) =
        resolve_object_root_path(pool, game_id, object_id).await?;
    // Toggle rename keeps identity, so one path-scoped entry covers both
    // spellings, through the async event tail after return.
    let _guard = watcher_state
        .suppressor
        .suppress_paths([current_absolute_path.as_str()]);
    let original_absolute_path = Path::new(&mods_path)
        .join(&object.folder_path)
        .to_string_lossy()
        .to_string();
    let current_path = Path::new(&current_absolute_path);
    if !current_path.exists() || !current_path.is_dir() {
        return Err(AppError::RuntimePathNotFound {
            target: current_absolute_path,
        });
    }

    // Disk is the source of truth: the rename is the whole mutation. Object
    // status, folder_path, child paths and the runtime projection converge
    // via the scoped InternalMutation reconcile the caller runs afterwards —
    // the single writer of those columns. Child mod status is NOT cascaded:
    // it derives from each mod's own folder name (see
    // `disk_reconcile::helpers::load_runtime_mod_metadata`), and the UI
    // derives EffectivelyDisabled from the ancestor chain.
    let Some(next_absolute_path) = rename_toggle_on_disk(current_path, enable, "object folder")?
    else {
        // Already in the requested state: the caller's reconcile re-syncs any
        // DB drift; a no-op needs nothing else.
        return Ok(ObjectSwitchOutcome {
            object_id: object_id.to_string(),
            original_path: original_absolute_path,
            next_path: current_absolute_path,
        });
    };

    Ok(ObjectSwitchOutcome {
        object_id: object_id.to_string(),
        original_path: original_absolute_path,
        next_path: next_absolute_path.to_string_lossy().to_string(),
    })
}
