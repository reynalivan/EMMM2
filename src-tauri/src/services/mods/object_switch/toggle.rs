//! Explicit enable/disable of an object root folder (Workspace Switch).

use super::resolve::resolve_object_root_path;
use crate::domain::errors::AppError;
use crate::domain::models::ItemStatus;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::core_ops::rename_toggle_on_disk;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;

pub struct ObjectSwitchOutcome {
    pub object_id: String,
    pub original_path: String,
    pub next_path: String,
}

/// Workspace Switch owns explicit object-root enable/disable.
/// Do not route object targets through mod-toggle services or Disk Reconcile.
pub async fn toggle_object_root_service(
    config: &crate::services::config::ConfigService,
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    op_lock: &OperationLock,
    game_id: &str,
    object_id: &str,
    enable: bool,
) -> Result<ObjectSwitchOutcome, AppError> {
    let _lock = op_lock.acquire().await?;
    let _guard = SuppressionGuard::new(&watcher_state.suppressor);

    let (object, mods_path, current_absolute_path) =
        resolve_object_root_path(pool, game_id, object_id).await?;
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

    let Some(next_absolute_path) = rename_toggle_on_disk(current_path, enable, "object folder")?
    else {
        crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
            pool,
            game_id,
            &[object_id.to_string()],
            false,
        )
        .await?;
        return Ok(ObjectSwitchOutcome {
            object_id: object_id.to_string(),
            original_path: original_absolute_path,
            next_path: current_absolute_path,
        });
    };

    let mods_root = Path::new(&mods_path);
    let old_relative_path = current_path
        .strip_prefix(mods_root)
        .unwrap_or(current_path)
        .to_string_lossy()
        .to_string();
    let new_relative_path = next_absolute_path
        .strip_prefix(mods_root)
        .unwrap_or(&next_absolute_path)
        .to_string_lossy()
        .to_string();

    let mut tx = pool.begin().await?;
    crate::repo::object_repo::update_object_runtime_folder_path(
        &mut *tx,
        game_id,
        &old_relative_path,
        &new_relative_path,
    )
    .await?;
    crate::repo::mod_repo::update_child_paths_tx(
        &mut tx,
        game_id,
        &old_relative_path,
        &new_relative_path,
        Some(&mods_path),
    )
    .await?;
    crate::repo::mod_repo::update_status_and_reason_for_object(
        &mut tx,
        game_id,
        &new_relative_path,
        if enable {
            ItemStatus::Enabled
        } else {
            ItemStatus::Disabled
        },
        if enable {
            None
        } else {
            Some(crate::common::corridor_constants::DISABLED_REASON_USER)
        },
    )
    .await?;
    crate::repo::object_repo::update_object_status(
        &mut *tx,
        object_id,
        if enable {
            ItemStatus::Enabled
        } else {
            ItemStatus::Disabled
        },
    )
    .await?;
    tx.commit().await?;

    crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
        pool,
        game_id,
        &[object_id.to_string()],
        false,
    )
    .await?;
    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        pool,
        config,
        watcher_state.suppressor.clone(),
        game_id,
        &[true, false],
        true,
        true,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(ObjectSwitchOutcome {
        object_id: object_id.to_string(),
        original_path: original_absolute_path,
        next_path: next_absolute_path.to_string_lossy().to_string(),
    })
}
