use crate::domain::errors::MetadataError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;

/// Set the category (Object Type) for a mod.
/// Updates the `mods` table.
pub async fn set_mod_category(
    config: &ConfigService,
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    category: &str,
) -> Result<(), MetadataError> {
    let canonical_path =
        PathGuard::validate_path(config, game_id, folder_path).map_err(MetadataError::Security)?;

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
        return Err(MetadataError::NotFound(
            "Mod not found in database. Please sync first.".to_string(),
        ));
    }

    Ok(())
}

/// Update the thumbnail for a mod folder.
/// Copies the source image to `preview.png` (or keeps extension) in the mod folder.
/// Invalidates cache.
pub fn update_mod_thumbnail(
    config: &ConfigService,
    game_id: &str,
    folder_path: &str,
    source_path: &str,
) -> Result<String, MetadataError> {
    let target_dir =
        PathGuard::validate_path(config, game_id, folder_path).map_err(MetadataError::Security)?;

    let source_path_obj = Path::new(source_path);
    if !source_path_obj.exists() || !source_path_obj.is_file() {
        return Err(MetadataError::NotFound(format!(
            "Source file does not exist: {source_path}"
        )));
    }

    // Determine the new thumbnail path within the mod folder
    let new_thumbnail_name = source_path_obj
        .file_name()
        .ok_or_else(|| MetadataError::Validation("Invalid source file name".to_string()))?
        .to_string_lossy()
        .to_string();
    let new_thumbnail_path = target_dir.join(&new_thumbnail_name);

    // Copy the source image to the mod folder
    fs::copy(source_path_obj, &new_thumbnail_path)?;

    // Invalidate cache for this mod's thumbnail
    ThumbnailCache::invalidate(&new_thumbnail_path);

    Ok(new_thumbnail_path.to_string_lossy().to_string())
}

pub async fn repair_orphan_mods(pool: &SqlitePool, game_id: &str) -> Result<usize, String> {
    let orphans = crate::repo::mod_repo::get_orphan_mods(pool, game_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    if orphans.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let mut repaired = 0usize;

    for row in &orphans {
        let mod_id = &row.id;
        let actual_name = &row.actual_name;
        let mod_folder_path = &row.folder_path;

        let clean_name =
            crate::services::scanner::core::normalizer::normalize_display_name(actual_name);

        let obj_folder = std::path::Path::new(mod_folder_path)
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| clean_name.clone());

        let mut new_objects_count = 0;
        let object_id = crate::services::scanner::sync::helpers::ensure_object_exists(
            &mut tx,
            game_id,
            &obj_folder,
            &clean_name,
            "Other",
            None,
            "[]",
            "{}",
            None,
            None,
            &mut new_objects_count,
        )
        .await
        .map_err(|e| e.to_string())?;

        crate::repo::mod_repo::update_mod_object_id_and_type_tx(
            &mut tx, mod_id, &object_id, "Other",
        )
        .await
        .map_err(|e| e.to_string())?;

        repaired += 1;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(repaired)
}

pub async fn toggle_favorite(
    config: &ConfigService,
    pool: &SqlitePool,
    watcher: &WatcherState,
    game_id: &str,
    folder_path: &str,
    favorite: bool,
) -> Result<(), MetadataError> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let full_path =
        PathGuard::validate_path(config, game_id, folder_path).map_err(MetadataError::Security)?;

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| MetadataError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = full_path
        .strip_prefix(base)
        .unwrap_or(&full_path)
        .to_string_lossy()
        .to_string();

    crate::repo::mod_repo::set_favorite_by_path(pool, game_id, &rel_path, favorite).await?;

    let update = crate::services::mods::info_json::ModInfoUpdate {
        is_favorite: Some(favorite),
        ..Default::default()
    };
    let _ = crate::services::mods::info_json::update_info_json(&full_path, &update);

    Ok(())
}

pub async fn toggle_mod_safe(
    config: &ConfigService,
    pool: &SqlitePool,
    watcher: &WatcherState,
    game_id: &str,
    folder_path: &str,
    safe: bool,
) -> Result<(), MetadataError> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let full_path =
        PathGuard::validate_path(config, game_id, folder_path).map_err(MetadataError::Security)?;

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| MetadataError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = full_path
        .strip_prefix(base)
        .unwrap_or(&full_path)
        .to_string_lossy()
        .to_string();

    // Update mod-level safety (is_safe lives on the mods table, not objects)
    crate::repo::mod_repo::set_mod_safe_by_path(pool, game_id, &rel_path, safe).await?;

    let update = crate::services::mods::info_json::ModInfoUpdate {
        is_safe: Some(safe),
        ..Default::default()
    };
    let _ = crate::services::mods::info_json::update_info_json(&full_path, &update);

    // Trigger Dirty State for BOTH safety states (old and new)
    // because the mod effectively "moved" corridors.
    let _ = crate::services::collection_service::handle_dirty_state(pool, game_id, safe).await;
    let _ = crate::services::collection_service::handle_dirty_state(pool, game_id, !safe).await;

    Ok(())
}

/// Toggle the pinned state of a single mod folder.
/// Ensures synchronization between DB and info.json on disk.
pub async fn toggle_pin(
    config: &ConfigService,
    pool: &SqlitePool,
    watcher: &WatcherState,
    game_id: &str,
    folder_path: &str,
    pin: bool,
) -> Result<(), MetadataError> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);
    let full_path =
        PathGuard::validate_path(config, game_id, folder_path).map_err(MetadataError::Security)?;

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| MetadataError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = full_path
        .strip_prefix(base)
        .unwrap_or(&full_path)
        .to_string_lossy()
        .to_string();

    crate::repo::mod_repo::set_pinned_by_path(pool, game_id, &rel_path, pin).await?;

    let update = crate::services::mods::info_json::ModInfoUpdate {
        is_pinned: Some(pin),
        ..Default::default()
    };
    let _ = crate::services::mods::info_json::update_info_json(&full_path, &update);

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

pub async fn suggest_random_mods(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Vec<RandomModProposal>, String> {
    use rand::seq::SliceRandom;

    let characters = crate::repo::object_repo::get_characters_for_game(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;

    if characters.is_empty() {
        return Ok(Vec::new());
    }

    let mut proposals = Vec::new();

    for (object_id, object_name) in characters {
        let mods = crate::repo::mod_repo::get_disabled_mods_by_object_id(pool, &object_id, is_safe)
            .await
            .map_err(|e| e.to_string())?;

        if mods.is_empty() {
            continue;
        }

        let candidates: Vec<(String, String, String)> = mods
            .into_iter()
            .filter_map(|row| {
                let path_obj = Path::new(&row.folder_path);
                if let Some(name) = path_obj.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        return None;
                    }
                }
                Some((
                    row.id.clone(),
                    row.actual_name.clone(),
                    row.folder_path.clone(),
                ))
            })
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
) -> Result<Vec<crate::services::scanner::conflict::ConflictInfo>, String> {
    let rows = crate::repo::mod_repo::get_enabled_mods_paths(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut ini_files: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    for path_str in rows {
        let path = Path::new(&path_str);
        if path.exists() {
            let content = crate::services::scanner::core::walker::scan_folder_content(path, 3);
            for ini in content.ini_files {
                ini_files.push((path.to_path_buf(), ini));
            }
        }
    }

    let conflicts = crate::services::scanner::conflict::detect_conflicts(&ini_files);
    Ok(conflicts)
}
