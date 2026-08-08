//! Enable/disable a mod folder on disk and keep the DB in sync.

use super::naming::{
    find_existing_sibling_case_insensitive, rename_conflict_error, standardize_prefix,
};
use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::{Path, PathBuf};

/// Map a rename failure to a structured error, surfacing the locking
/// processes when the folder is busy.
pub(crate) fn map_toggle_error(src: &Path, noun: &str, error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        let processes = crate::services::fs_utils::locking::get_locking_processes(src);
        if !processes.is_empty() {
            return AppError::FileInUse {
                path: src.to_string_lossy().to_string(),
                processes,
            };
        }

        return AppError::PathBusy {
            path: src.to_string_lossy().to_string(),
        };
    }

    AppError::Io(format!("Failed to rename {noun}: {error}"))
}

/// Rename `src` to its enabled/disabled form on disk.
/// Returns `Ok(None)` when the folder already has the desired prefix state.
pub(crate) fn rename_toggle_on_disk(
    src: &Path,
    enable: bool,
    noun: &str,
) -> Result<Option<PathBuf>, AppError> {
    let old_name = src.file_name().unwrap_or_default().to_string_lossy();
    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        return Ok(None);
    }

    let parent = src
        .parent()
        .ok_or_else(|| AppError::Io("Invalid path".to_string()))?;
    let new_path = parent.join(&new_name);

    // Guard: target already exists → rename collision (both X and DISABLED X on disk)
    if let Some(existing_path) = find_existing_sibling_case_insensitive(parent, &new_name, src) {
        let base = crate::common::normalizer::normalize_display_name(&old_name);
        return Err(rename_conflict_error(&new_path, &existing_path, &base));
    }

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(src, &new_path)
        .map_err(|error| map_toggle_error(src, noun, error))?;

    Ok(Some(new_path))
}

/// After a top-level folder rename, point the object row and all child mod
/// rows at the new path. Non-fatal: the disk rename already happened.
pub(crate) async fn sync_object_and_child_paths(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
    old_rel: &str,
    new_rel: &str,
) {
    if Path::new(old_rel).components().count() != 1 {
        return;
    }

    let _ =
        crate::repo::object_repo::update_object_folder_path(pool, game_id, old_rel, new_rel).await;

    if let Err(e) =
        crate::repo::mod_repo::update_child_paths(pool, game_id, old_rel, new_rel, Some(mods_path))
            .await
    {
        log::warn!("Failed to update child paths ({old_rel} -> {new_rel}): {e}");
    }
}

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

    let Some(new_path) = rename_toggle_on_disk(src, enable, "mod folder")? else {
        return Ok(path);
    };

    log::info!(
        "Toggled mod: '{}' -> '{}'",
        src.file_name().unwrap_or_default().to_string_lossy(),
        new_path.display()
    );

    Ok(new_path.to_string_lossy().to_string())
}

#[allow(clippy::too_many_arguments)] // Service boundary kept stable to preserve toggle and duplicate-resolution callers.
pub async fn toggle_mod_inner_service_with_duplicate_policy(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    path: &ValidatedPath,
    enable: bool,
    game_id: &str,
    allow_duplicates: bool,
) -> Result<String, AppError> {
    let canonical_path = path;

    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Failed to fetch game mods path".to_string()))?;

    let base = Path::new(&mods_path);
    let rel_path = canonical_path
        .strip_prefix(base)
        .unwrap_or(canonical_path)
        .to_string_lossy()
        .to_string();
    let mut changed_object_ids = Vec::new();
    if let Some((_, Some(object_id), _)) =
        crate::repo::mod_repo::get_mod_id_and_status_by_path(pool, &rel_path, game_id).await?
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

    // Same value as `rel_path` above — the folder has not moved yet.
    let old_rel = rel_path.as_str();
    let new_abs = Path::new(&new_absolute_path);
    let new_rel = new_abs
        .strip_prefix(base)
        .unwrap_or(new_abs)
        .to_string_lossy()
        .to_string();

    crate::repo::mod_repo::update_mod_path_status_and_reason(
        pool,
        game_id,
        old_rel,
        &new_rel,
        new_status,
        disabled_reason,
    )
    .await?;

    // Update object folder_path and child paths if this is a top-level folder
    sync_object_and_child_paths(pool, game_id, &mods_path, old_rel, &new_rel).await;

    let is_safe = crate::repo::mod_repo::get_is_safe_by_folder(pool, game_id, &new_rel)
        .await
        .ok()
        .flatten();

    if is_safe.is_some() {
        crate::services::app::runtime_effects::finalize_mutation(
            pool,
            config,
            game_id,
            crate::services::app::runtime_effects::MutationOutcome::objects(changed_object_ids),
        )
        .await;
    }

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
        sync_object_and_child_paths(pool, game_id, mods_path, rel_path, &new_rel).await;
    }
    Ok(new_abs)
}
