//! Bulk attribute updates: info.json fields plus favorite/pin flags.

use super::types::{BulkActionError, BulkResult};
use crate::services::mods::info_json;
use sqlx::SqlitePool;

pub async fn bulk_update_info(
    config: &crate::services::config::ConfigService,
    game_id: &str,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let mut success = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        let canonical =
            crate::services::fs_utils::guard::PathGuard::validate_path(config, game_id, &path)
                .map_err(crate::domain::errors::AppError::Security)?;

        match info_json::update_info_json(&canonical, &update) {
            Ok(_) => success.push(path),
            Err(e) => failures.push(BulkActionError {
                path,
                error: crate::domain::errors::AppError::Metadata(e),
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
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let failures = Vec::new();
    let mut relatives = Vec::new();

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, &game_id)
        .await?
        .ok_or_else(|| {
            crate::domain::errors::AppError::NotFound(
                "Game not found or has no mods_path".to_string(),
            )
        })?;

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in &folder_paths {
        let rel_path = std::path::Path::new(folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(folder_path))
            .to_string_lossy()
            .to_string();

        relatives.push(rel_path);
    }

    if let Err(e) =
        crate::repo::mod_repo::batch_set_favorite(pool, &game_id, &relatives, favorite).await
    {
        return Err(crate::domain::errors::AppError::Io(e.to_string()));
    }

    // Opt-R: Parallel info.json writes using rayon
    use rayon::prelude::*;
    let update_for_parallel = info_json::ModInfoUpdate {
        is_favorite: Some(favorite),
        ..Default::default()
    };
    folder_paths.par_iter().for_each(|folder_path| {
        let full_path = std::path::Path::new(folder_path);
        if full_path.exists() {
            let _ = info_json::update_info_json(full_path, &update_for_parallel);
        }
    });
    Ok(BulkResult::new(folder_paths, failures))
}

pub async fn bulk_pin(
    pool: &SqlitePool,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<BulkResult, crate::domain::errors::AppError> {
    let failures = Vec::new();
    let mut relatives = Vec::new();

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, &game_id)
        .await?
        .ok_or_else(|| {
            crate::domain::errors::AppError::NotFound(
                "Game not found or has no mods_path".to_string(),
            )
        })?;

    let base = std::path::Path::new(&game_mod_path);

    for folder_path in &folder_paths {
        let rel_path = std::path::Path::new(folder_path)
            .strip_prefix(base)
            .unwrap_or(std::path::Path::new(folder_path))
            .to_string_lossy()
            .to_string();

        relatives.push(rel_path);
    }

    if let Err(e) = crate::repo::mod_repo::batch_set_pinned(pool, &game_id, &relatives, pin).await {
        return Err(crate::domain::errors::AppError::Io(e.to_string()));
    }

    Ok(BulkResult::new(folder_paths, failures))
}
