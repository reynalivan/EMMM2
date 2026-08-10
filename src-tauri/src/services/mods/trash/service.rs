//! Service-level soft delete: path validation, DB cleanup and runtime effects.

use super::store::move_to_trash;
use super::types::DeleteModResult;
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::scanner::watcher::WatcherState;
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
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    trash_dir: std::path::PathBuf,
    path: &ValidatedPath,
    game_id: &str,
) -> Result<DeleteModResult, AppError> {
    let original = path.original();
    let (is_safe, object_id, relative_path) = {
        let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
            .await
            .ok()
            .flatten();

        if let Some(mp) = mods_path {
            let base = Path::new(&mp);
            let rel = Path::new(original)
                .strip_prefix(base)
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|_| original.to_string());

            let safe = crate::repo::mod_repo::get_is_safe_by_folder(pool, game_id, &rel)
                .await
                .ok()
                .flatten();
            let object =
                crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, &rel, game_id)
                    .await
                    .ok()
                    .flatten();
            (safe, object, Some(rel))
        } else {
            (None, None, None)
        }
    };

    let _guard = state.suppressor.suppress_paths([original]);

    move_to_trash(Path::new(original), &trash_dir, Some(game_id.to_string()))?;
    let _ = crate::repo::mod_repo::delete_mod_by_path(pool, original).await;
    let collection_impact = if let Some(rel) = relative_path.as_deref() {
        crate::services::collection_service::handle_mod_missing(pool, rel)
            .await
            .unwrap_or_default()
    } else {
        CollectionReferenceImpact::default()
    };

    if is_safe.is_some() {
        let changed_object_ids = object_id.into_iter().collect::<Vec<_>>();
        crate::services::app::runtime_effects::finalize_mutation(
            pool,
            config,
            game_id,
            crate::services::app::runtime_effects::MutationOutcome::objects(changed_object_ids),
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
    let _guard = state.suppressor.suppress_paths([path_obj]);
    move_to_trash(path_obj, trash_dir, game_id).map(|_| ())
}
