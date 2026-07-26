//! Enable/disable a mod folder on disk and keep the DB in sync.

use super::naming::{
    find_existing_sibling_case_insensitive, rename_conflict_error, standardize_prefix,
};
use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;

pub async fn toggle_mod_inner(
    state: &WatcherState,
    path: String,
    enable: bool,
) -> Result<String, AppError> {
    // Hold suppression for the entire function so watcher events don't
    // leak through between the fs::rename and function return.
    let _guard = SuppressionGuard::new(&state.suppressor);

    let src = Path::new(&path);
    if !src.exists() || !src.is_dir() {
        return Err(AppError::Io(format!("Mod folder does not exist: {path}")));
    }

    let parent = src
        .parent()
        .ok_or_else(|| AppError::Io("Invalid path".to_string()))?;
    let old_name = src.file_name().unwrap_or_default().to_string_lossy();

    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        return Ok(path);
    }

    let new_path = parent.join(&new_name);

    // Guard: target already exists → rename collision (both X and DISABLED X on disk)
    if let Some(existing_path) = find_existing_sibling_case_insensitive(parent, &new_name, src) {
        let base = crate::common::normalizer::normalize_display_name(&old_name);
        return Err(rename_conflict_error(&new_path, &existing_path, &base));
    }

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, &new_path).map_err(
        |e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let processes = crate::services::fs_utils::locking::get_locking_processes(src);
                if !processes.is_empty() {
                    return AppError::FileInUse {
                        path: path.clone(),
                        processes,
                    };
                }

                return AppError::PathBusy { path: path.clone() };
            }
            AppError::Io(format!("Failed to rename mod folder: {e}"))
        },
    )?;

    log::info!("Toggled mod: '{}' -> '{}'", old_name, new_path.display());

    Ok(new_path.to_string_lossy().to_string())
}

#[allow(clippy::too_many_arguments)] // Service boundary kept stable to preserve toggle and duplicate-resolution callers.
pub async fn toggle_mod_inner_service_with_duplicate_policy(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    path: String,
    enable: bool,
    game_id: &str,
    allow_duplicates: bool,
) -> Result<String, AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Io)?;

    let canonical_path =
        validate_path(config, game_id, &path).map_err(AppError::Security)?;

    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Failed to fetch game mods path".to_string()))?;

    let base = Path::new(&mods_path);
    let rel_path = canonical_path
        .strip_prefix(base)
        .unwrap_or(&canonical_path)
        .to_string_lossy()
        .to_string();
    let mut changed_object_ids = Vec::new();
    if let Some((_, Some(object_id), _)) =
        crate::repo::mod_repo::get_mod_id_and_status_by_path_any(pool, &rel_path, game_id).await?
    {
        changed_object_ids.push(object_id);
    }

    // AC-29.1: Conflict Detection
    if enable && !allow_duplicates {
        let duplicates: Vec<crate::domain::mods::DuplicateModInfo> =
            crate::services::scanner::conflict::get_duplicates_for_mod_service(
                pool, &rel_path, game_id,
            )
            .await?;

        if !duplicates.is_empty() {
            // Implicit Swap: If ALL duplicates are variants, auto-disable them
            let all_variants = duplicates.iter().all(|d| d.is_variant);
            if all_variants {
                for duplicate in &duplicates {
                    changed_object_ids.push(duplicate.object_id.clone());
                }
                for dup in duplicates {
                    let _ = toggle_and_sync_db(
                        pool,
                        state,
                        &mods_path,
                        game_id,
                        &dup.mod_id,
                        &dup.folder_path,
                        false,
                    )
                    .await?;
                }
            } else {
                // Real conflict -> Signal frontend to show radio resolution modal
                return Err(AppError::DuplicateConflict(duplicates));
            }
        }
    }

    let new_absolute_path =
        toggle_mod_inner(state, canonical_path.to_string_lossy().to_string(), enable).await?;
    let new_status = if enable {
        crate::domain::models::ItemStatus::Enabled
    } else {
        crate::domain::models::ItemStatus::Disabled
    };

    let disabled_reason = if enable {
        None
    } else {
        Some(crate::common::corridor_constants::DISABLED_REASON_USER)
    };

    let old_rel = canonical_path
        .strip_prefix(base)
        .unwrap_or(&canonical_path)
        .to_string_lossy()
        .to_string();
    let new_abs = Path::new(&new_absolute_path);
    let new_rel = new_abs
        .strip_prefix(base)
        .unwrap_or(new_abs)
        .to_string_lossy()
        .to_string();

    crate::repo::mod_repo::update_mod_path_status_and_reason(
        pool,
        game_id,
        &old_rel,
        &new_rel,
        new_status,
        disabled_reason,
    )
    .await?;

    // Update object folder_path and child paths if this is a top-level folder
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
            log::warn!("Failed to update child paths (backslash) after toggle: {e}");
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
            log::warn!("Failed to update child paths (forward-slash) after toggle: {e}");
        }
    }

    // Recompute corridor signature so dirty detection works
    let is_safe = crate::repo::mod_repo::get_is_safe_by_folder(pool, game_id, &new_rel)
        .await
        .ok()
        .flatten();

    if let Some(is_safe_bool) = is_safe {
        if let Err(e) =
            crate::services::corridor_service::recompute_signature(pool, game_id, is_safe_bool)
                .await
        {
            log::warn!("Failed to recompute corridor signature after toggle: {e}");
        }

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

    crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
        pool,
        game_id,
        &changed_object_ids,
        true,
    )
    .await?;

    Ok(new_absolute_path)
}

/// Toggle a mod on disk and sync all DB state (path, object, children).
/// Used by privacy corridor handoff and single-mod toggle.
pub async fn toggle_and_sync_db(
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    mods_path: &str,
    game_id: &str,
    id: &str,
    rel_path: &str,
    enable: bool,
) -> Result<String, AppError> {
    let abs_path = Path::new(mods_path)
        .join(rel_path)
        .to_string_lossy()
        .to_string();
    let new_abs = toggle_mod_inner(watcher_state, abs_path, enable).await?;

    let new_rel = Path::new(&new_abs)
        .strip_prefix(mods_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| new_abs.clone());

    if new_rel != rel_path {
        let _ = crate::repo::mod_repo::update_mod_path_by_id(pool, id, &new_rel).await;

        // Top-level folder → also update object + children
        let rel_components: Vec<_> = Path::new(rel_path).components().collect();
        if rel_components.len() == 1 {
            let _ = crate::repo::object_repo::update_object_folder_path(
                pool, game_id, rel_path, &new_rel,
            )
            .await;
            for (old_sep, new_sep) in [("\\", "\\"), ("/", "/")] {
                let _ = crate::repo::mod_repo::update_child_paths(
                    pool,
                    game_id,
                    &format!("{}{}", rel_path, old_sep),
                    &format!("{}{}", new_rel, new_sep),
                    Some(mods_path),
                )
                .await;
            }
        }
    }
    Ok(new_abs)
}
