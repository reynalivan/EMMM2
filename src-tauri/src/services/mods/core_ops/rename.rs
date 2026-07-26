//! Rename a mod folder on disk and cascade the new path through DB and collections.

use super::naming::{find_existing_sibling_case_insensitive, rename_conflict_error};
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub new_name: String,
    pub collection_impact: CollectionReferenceImpact,
}

pub async fn rename_mod_folder_inner(
    state: &WatcherState,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, AppError> {
    // Hold suppression for the entire function so watcher events don't
    // leak through between the fs::rename and function return.
    let _guard = SuppressionGuard::new(&state.suppressor);

    let path = Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(AppError::Io(format!(
            "Folder does not exist: {folder_path}"
        )));
    }

    if new_name.is_empty() || new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(AppError::Io(
            "Invalid folder name — contains reserved characters".to_string(),
        ));
    }

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Io("Cannot determine parent directory".to_string()))?;
    let old_folder_name = path
        .file_name()
        .ok_or_else(|| AppError::Io("Invalid folder name".to_string()))?
        .to_string_lossy()
        .to_string();

    let new_folder_name = if crate::common::normalizer::is_disabled_folder(&old_folder_name) {
        format!("{}{}", crate::DISABLED_PREFIX, new_name)
    } else {
        new_name.clone()
    };

    let new_path = parent.join(&new_folder_name);
    if let Some(existing_path) =
        find_existing_sibling_case_insensitive(parent, &new_folder_name, path)
    {
        let base_name = crate::common::normalizer::normalize_display_name(&old_folder_name);
        return Err(rename_conflict_error(&new_path, &existing_path, &base_name));
    }

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(path, &new_path).map_err(
        |e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let processes = crate::services::fs_utils::locking::get_locking_processes(path);
                if !processes.is_empty() {
                    return AppError::FileInUse {
                        path: folder_path.clone(),
                        processes,
                    };
                }

                return AppError::PathBusy {
                    path: folder_path.clone(),
                };
            }
            AppError::Io(format!("Failed to rename folder: {e}"))
        },
    )?;

    update_info_json_name(&new_path, &new_name);

    log::info!("Renamed '{}' -> '{}'", old_folder_name, new_folder_name);

    Ok(RenameResult {
        old_path: folder_path,
        new_path: new_path.to_string_lossy().to_string(),
        new_name,
        collection_impact: CollectionReferenceImpact::default(),
    })
}

fn update_info_json_name(folder_path: &Path, new_name: &str) {
    use crate::services::mods::info_json;
    if folder_path.join("info.json").exists() {
        let update = info_json::ModInfoUpdate {
            actual_name: Some(new_name.to_string()),
            ..Default::default()
        };
        let _ = info_json::update_info_json(folder_path, &update);
    }
}

pub async fn rename_mod_folder_inner_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    old_path: String,
    new_name: String,
    game_id: &str,
) -> Result<RenameResult, AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Io)?;

    let canonical_path =
        validate_path(config, game_id, &old_path).map_err(AppError::Security)?;

    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Failed to fetch game mods path".to_string()))?;

    let base = Path::new(&mods_path);

    // AC-21.1.6: Windows path limit check (260 characters)
    #[cfg(target_os = "windows")]
    {
        let parent = canonical_path.parent().unwrap_or_else(|| Path::new(""));
        let new_abs_path = parent.join(&new_name);
        let path_str = new_abs_path.to_string_lossy();
        if path_str.len() >= 260 {
            return Err(AppError::Io(format!(
                "Windows path limit exceeded ({} chars). Please use a shorter name.",
                path_str.len()
            )));
        }
    }

    let mut result = rename_mod_folder_inner(
        state,
        canonical_path.to_string_lossy().to_string(),
        new_name.clone(),
    )
    .await?;
    let new_absolute_path = &result.new_path;

    let old_rel = canonical_path
        .strip_prefix(base)
        .unwrap_or(&canonical_path)
        .to_string_lossy()
        .to_string();

    let new_abs = Path::new(new_absolute_path);
    let new_rel = new_abs
        .strip_prefix(base)
        .unwrap_or(new_abs)
        .to_string_lossy()
        .to_string();

    if let Err(e) = crate::repo::mod_repo::update_mod_path_by_old_path_in_game(
        pool, game_id, &old_rel, &new_rel,
    )
    .await
    {
        log::warn!("Failed to update mod path in DB after rename ({old_rel} -> {new_rel}): {e}");
    }

    // Collection Auto-Healing: cascade path changes to all saved collections
    result.collection_impact = crate::services::collection_service::handle_mod_moved_or_renamed(
        pool, &old_rel, &new_rel, None,
    )
    .await
    .unwrap_or_default();

    let rel_components: Vec<_> = Path::new(&old_rel).components().collect();
    if rel_components.len() == 1 {
        let _ =
            crate::repo::object_repo::update_object_folder_path(pool, game_id, &old_rel, &new_rel)
                .await;

        let old_prefix = format!("{}\\", old_rel);
        let new_prefix = format!("{}\\", new_rel);
        let old_prefix_fwd = format!("{}/", old_rel);
        let new_prefix_fwd = format!("{}/", new_rel);

        if let Err(e) = crate::repo::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_prefix,
            &new_prefix,
            Some(&mods_path),
        )
        .await
        {
            log::warn!("Failed to update child paths (backslash) after rename: {e}");
        }

        if let Err(e) = crate::repo::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_prefix_fwd,
            &new_prefix_fwd,
            Some(&mods_path),
        )
        .await
        {
            log::warn!("Failed to update child paths (forward-slash) after rename: {e}");
        }
    }

    let is_safe = crate::repo::mod_repo::get_is_safe_by_folder(pool, game_id, &new_rel)
        .await
        .ok()
        .flatten();

    if let Some(is_safe_bool) = is_safe {
        let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
            pool,
            config,
            state.suppressor.clone(),
            game_id,
            &[is_safe_bool],
            true,
            true,
        )
        .await;
    }

    Ok(result)
}
