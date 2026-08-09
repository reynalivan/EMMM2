//! Enable/disable a mod folder on disk and keep the DB in sync.

use super::naming::{
    find_existing_sibling_case_insensitive, rename_conflict_error, standardize_prefix,
};
use crate::domain::errors::AppError;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::scanner::watcher::WatcherState;
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
    // Path-scoped: covers both spellings of the rename (same identity key)
    // and keeps suppressing through the async event tail after return.
    let _guard = state.suppressor.suppress_paths([&path]);

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

/// What a policy-checked toggle changed on disk.
pub struct ModTogglePolicyOutcome {
    pub new_absolute_path: String,
    /// Sibling variants the implicit swap auto-disabled (absolute paths, both
    /// spellings). They can live under other object roots, so the caller's
    /// reconcile scope must include them explicitly.
    pub swapped_paths: Vec<String>,
}

#[allow(clippy::too_many_arguments)] // Service boundary kept stable to preserve toggle and duplicate-resolution callers.
pub async fn toggle_mod_inner_service_with_duplicate_policy(
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    path: &ValidatedPath,
    enable: bool,
    game_id: &str,
    allow_duplicates: bool,
) -> Result<ModTogglePolicyOutcome, AppError> {
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
    // AC-29.1: Conflict Detection
    let mut swapped_paths = Vec::new();
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
                for dup in duplicates {
                    let dup_abs = Path::new(&mods_path)
                        .join(&dup.folder_path)
                        .to_string_lossy()
                        .to_string();
                    let dup_new = toggle_mod_inner(state, dup_abs.clone(), false).await?;
                    swapped_paths.push(dup_abs);
                    swapped_paths.push(dup_new);
                }
            } else {
                // Real conflict -> Signal frontend to show radio resolution modal
                return Err(AppError::DuplicateConflict(duplicates));
            }
        }
    }

    // Disk is the source of truth: the rename is the whole mutation. The DB
    // (status, folder_path, projection) converges via the scoped
    // InternalMutation reconcile the caller runs afterwards — the single
    // writer of those columns.
    let new_absolute_path =
        toggle_mod_inner(state, canonical_path.to_string_lossy().to_string(), enable).await?;

    Ok(ModTogglePolicyOutcome {
        new_absolute_path,
        swapped_paths,
    })
}
