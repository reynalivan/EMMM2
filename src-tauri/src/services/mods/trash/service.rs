//! Service-level soft delete: path validation, DB cleanup and runtime effects.

use super::store::move_to_trash;
use super::types::DeleteModResult;
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::fs;
use std::path::Path;

/// Move a mod folder to the trash directory.
///
/// Creates `{trash_dir}/{uuid}/` containing:
/// - The original folder contents
/// - `metadata.json` with restore information
///
/// Returns the `TrashMetadata` on success.
pub async fn delete_mod_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    trash_dir: std::path::PathBuf,
    path: String,
    game_id: Option<String>,
) -> Result<DeleteModResult, AppError> {
    let _lock = op_lock.acquire().await?;

    if !trash_dir.exists() {
        fs::create_dir_all(&trash_dir)
            .map_err(|e| AppError::Io(format!("Failed to create trash dir: {}", e)))?;
    }

    if let Some(ref gid) = game_id {
        validate_path(config, gid, &path)?;
    }

    let (is_safe, object_id, relative_path) = if let Some(ref gid) = game_id {
        let mods_path = crate::repo::game_repo::get_mod_path(pool, gid)
            .await
            .ok()
            .flatten();

        if let Some(mp) = mods_path {
            let base = Path::new(&mp);
            let rel = Path::new(&path)
                .strip_prefix(base)
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());

            let safe = crate::repo::mod_repo::get_is_safe_by_folder(pool, gid, &rel)
                .await
                .ok()
                .flatten();
            let object = crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, &rel, gid)
                .await
                .ok()
                .flatten();
            (safe, object, Some(rel))
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    let path_obj = Path::new(&path);
    let _guard = SuppressionGuard::new(&state.suppressor);

    move_to_trash(path_obj, &trash_dir, game_id.clone())?;
    let _ = crate::repo::mod_repo::delete_mod_by_path(pool, &path).await;
    let collection_impact = if let Some(rel) = relative_path.as_deref() {
        crate::services::collection_service::handle_mod_missing(pool, rel)
            .await
            .unwrap_or_default()
    } else {
        CollectionReferenceImpact::default()
    };

    if let (Some(gid), Some(safe)) = (game_id, is_safe) {
        let changed_object_ids = object_id.into_iter().collect::<Vec<_>>();
        let _ = crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
            pool,
            &gid,
            &changed_object_ids,
            false,
        )
        .await;
        let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
            pool,
            config,
            state.suppressor.clone(),
            &gid,
            &[safe],
            true,
            true,
        )
        .await;
    }

    Ok(DeleteModResult { collection_impact })
}

/// Helper that suppresses the watcher for the single move action.
pub async fn move_to_trash_guarded(
    state: &WatcherState,
    trash_dir: &Path,
    path: String,
    game_id: Option<String>,
) -> Result<(), AppError> {
    let path_obj = Path::new(&path);
    let _guard = SuppressionGuard::new(&state.suppressor);
    move_to_trash(path_obj, trash_dir, game_id).map(|_| ())
}
