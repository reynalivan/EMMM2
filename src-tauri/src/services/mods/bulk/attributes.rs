//! Bulk attribute updates: info.json fields plus favorite/pin flags.

use super::types::{BulkActionError, BulkResult};
use crate::domain::errors::AppError;
use crate::services::mods::info_json;
use sqlx::SqlitePool;
use std::path::Path;

pub async fn bulk_update_info(
    config: &crate::services::config::ConfigService,
    game_id: &str,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, AppError> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        let canonical = crate::services::fs_utils::guard::validate_path(config, game_id, &path)?;

        match info_json::update_info_json(&canonical, &update) {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError {
                path,
                error: AppError::Metadata(e),
            }),
        }
    }
    Ok(BulkResult::new(success, failures))
}

pub async fn bulk_toggle_favorite(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<BulkResult, AppError> {
    let relatives = relative_to_mods_root(pool, &game_id, &folder_paths).await?;

    crate::repo::mod_repo::batch_set_favorite(pool, &game_id, &relatives, favorite)
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;

    // Opt-R: Parallel info.json writes using rayon
    let update = info_json::ModInfoUpdate {
        is_favorite: Some(favorite),
        ..Default::default()
    };
    Ok(partition_info_json_writes(folder_paths, &update))
}

pub async fn bulk_pin(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<BulkResult, AppError> {
    let relatives = relative_to_mods_root(pool, &game_id, &folder_paths).await?;

    crate::repo::mod_repo::batch_set_pinned(pool, &game_id, &relatives, pin)
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;

    Ok(BulkResult::new(folder_paths, Vec::new()))
}

/// Mod paths relative to the game's mods root, the form the DB stores.
async fn relative_to_mods_root(
    pool: &SqlitePool,
    game_id: &str,
    folder_paths: &[String],
) -> Result<Vec<String>, AppError> {
    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = Path::new(&game_mod_path);
    Ok(folder_paths
        .iter()
        .map(|folder_path| {
            let path = Path::new(folder_path);
            path.strip_prefix(base)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string()
        })
        .collect())
}

/// Mirror a flag into every mod's info.json in parallel, splitting the batch into
/// written and not-written. A folder that vanished mid-batch (renamed by a toggle,
/// deleted on disk) lands in `failures`: the DB row is already flagged, so dropping
/// the write error would leave disk and DB disagreeing with nothing to show for it.
fn partition_info_json_writes(
    folder_paths: Vec<String>,
    update: &info_json::ModInfoUpdate,
) -> BulkResult {
    use rayon::prelude::*;

    let (success, failures): (Vec<_>, Vec<_>) = folder_paths
        .into_par_iter()
        .map(
            |folder_path| match info_json::update_info_json(Path::new(&folder_path), update) {
                Ok(_) => Ok(folder_path),
                Err(e) => Err(BulkActionError {
                    path: folder_path,
                    error: AppError::Metadata(e),
                }),
            },
        )
        .partition(Result::is_ok);

    BulkResult::new(
        success.into_iter().map(Result::unwrap).collect(),
        failures.into_iter().map(Result::unwrap_err).collect(),
    )
}

#[cfg(test)]
#[path = "../tests/bulk_attributes_tests.rs"]
mod tests;
