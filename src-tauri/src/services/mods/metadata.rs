use crate::domain::errors::AppError;
use crate::domain::models::ItemStatus;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;

/// Set the category (Object Type) for a mod.
/// Updates the `mods` table.
pub async fn set_mod_category(
    pool: &SqlitePool,
    game_id: &str,
    canonical_path: &ValidatedPath,
    category: &str,
) -> Result<(), AppError> {
    let folder_path_str = canonical_path.to_string_lossy();

    let exists =
        crate::repo::mod_repo::get_mod_id_and_object_id_by_path(pool, &folder_path_str, game_id)
            .await?;

    if let Some((mod_id, object_id)) = exists {
        let obj_id_str = object_id.unwrap_or_default();
        let mut conn = pool.acquire().await?;

        crate::repo::mod_repo::update_mod_object_id_and_type_tx(
            &mut conn,
            &mod_id,
            &obj_id_str,
            category,
        )
        .await?;
    } else {
        return Err(AppError::NotFound(
            "Mod not found in database. Please sync first.".to_string(),
        ));
    }

    Ok(())
}

/// Update the thumbnail for a mod folder.
/// Copies the source image to `preview.png` (or keeps extension) in the mod folder.
/// Invalidates cache.
pub fn update_mod_thumbnail(
    target_dir: &ValidatedPath,
    source_path: &str,
) -> Result<String, AppError> {
    let source_path_obj = Path::new(source_path);
    if !source_path_obj.exists() || !source_path_obj.is_file() {
        return Err(AppError::NotFound(format!(
            "Source file does not exist: {source_path}"
        )));
    }

    // Determine the new thumbnail path within the mod folder
    let new_thumbnail_name = source_path_obj
        .file_name()
        .ok_or_else(|| AppError::Validation("Invalid source file name".to_string()))?
        .to_string_lossy()
        .to_string();
    let new_thumbnail_path = target_dir.join(&new_thumbnail_name);

    // Copy the source image to the mod folder
    fs::copy(source_path_obj, &new_thumbnail_path)?;

    // Invalidate cache for this mod's thumbnail
    ThumbnailCache::invalidate(&new_thumbnail_path);

    Ok(new_thumbnail_path.to_string_lossy().to_string())
}

pub async fn toggle_mod_safe(
    config: &ConfigService,
    pool: &SqlitePool,
    watcher: &WatcherState,
    game_id: &str,
    full_path: &ValidatedPath,
    safe: bool,
) -> Result<(), AppError> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = full_path
        .strip_prefix(base)
        .unwrap_or(full_path)
        .to_string_lossy()
        .to_string();

    // Update mod-level safety (is_safe lives on the mods table, not objects)
    let object_id =
        crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, &rel_path, game_id).await?;
    crate::repo::mod_repo::set_mod_safe_by_path(pool, game_id, &rel_path, safe).await?;

    let update = crate::services::mods::info_json::ModInfoUpdate {
        is_safe: Some(safe),
        ..Default::default()
    };
    let _ = crate::services::mods::info_json::update_info_json(full_path, &update);

    crate::services::app::runtime_effects::finalize_mutation(
        pool,
        config,
        game_id,
        crate::services::app::runtime_effects::MutationOutcome::objects(object_id),
    )
    .await;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct RandomModProposal {
    pub object_id: String,
    pub object_name: String,
    pub mod_id: String,
    pub name: String,
    pub thumbnail_path: Option<String>,
    pub folder_path: String,
}

fn path_has_hidden_segment(path: &str) -> bool {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(|segment| segment.starts_with('.'))
}

fn path_has_disabled_segment(path: &str) -> bool {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(crate::common::normalizer::is_disabled_folder)
}

fn is_effectively_disabled_randomizer_candidate(mod_row: &crate::repo::mod_repo::Mod) -> bool {
    if path_has_hidden_segment(&mod_row.folder_path) {
        return false;
    }

    mod_row.status == ItemStatus::Disabled || path_has_disabled_segment(&mod_row.folder_path)
}

pub async fn suggest_random_mods(
    pool: &SqlitePool,
    game_id: &str,
    corridor: crate::domain::corridor::Corridor,
) -> Result<Vec<RandomModProposal>, AppError> {
    let is_safe = corridor.is_safe();
    use rand::seq::SliceRandom;

    let characters = crate::repo::object_repo::get_characters_for_game(pool, game_id).await?;

    if characters.is_empty() {
        return Ok(Vec::new());
    }

    let mut proposals = Vec::new();

    for (object_id, object_name) in characters {
        let mods = crate::repo::mod_repo::get_mods_by_object_id(pool, &object_id, is_safe).await?;

        if mods.is_empty() {
            continue;
        }

        let candidates: Vec<(String, String, String)> = mods
            .into_iter()
            .filter(is_effectively_disabled_randomizer_candidate)
            .map(|row| (row.id, row.actual_name, row.folder_path))
            .collect();

        let mut rng = rand::thread_rng();
        if let Some((mod_id, name, path)) = candidates.choose(&mut rng) {
            proposals.push(RandomModProposal {
                object_id: object_id.clone(),
                object_name: object_name.clone(),
                mod_id: mod_id.clone(),
                name: name.clone(),
                thumbnail_path: None,
                folder_path: path.clone(),
            });
        }
    }

    Ok(proposals)
}

pub async fn get_active_mod_conflicts(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, AppError> {
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} has no mods path")))?;
    let rows = crate::repo::mod_repo::get_enabled_mods_paths(pool, game_id).await?;

    Ok(conflicts_for_enabled_paths(Path::new(&mods_path), &rows))
}

/// Conflict detection over an enabled-mod path list the caller already has.
///
/// Post-apply needs both the conflicts and the same path list for its harvest;
/// without this it issued the identical query twice.
///
/// `enabled_paths` are `mods.folder_path` values, which disk reconcile writes
/// relative to the mods root. This used to test them with `Path::exists`
/// directly: a relative path resolves against the process working directory,
/// so every such row failed the check and was skipped, and the whole feature
/// reported "no conflicts" without ever reading a file. `join` handles both
/// conventions -- an absolute argument replaces the base -- which matters
/// while the scanner commit still writes absolute paths.
pub fn conflicts_for_enabled_paths(
    mods_root: &Path,
    enabled_paths: &[String],
) -> Vec<crate::services::scanner::conflict::ConflictInfo> {
    let mut ini_files: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    for path_str in enabled_paths {
        let path = mods_root.join(path_str);
        if !path.exists() {
            continue;
        }
        let content = crate::services::scanner::core::walker::scan_folder_content(&path, 3);
        for ini in content.ini_files {
            ini_files.push((path.clone(), ini));
        }
    }

    crate::services::scanner::conflict::detect_conflicts(&ini_files)
}

#[cfg(test)]
#[path = "tests/metadata_conflict_tests.rs"]
mod metadata_conflict_tests;
