use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn standardize_prefix(folder_name: &str, target_enabled: bool) -> String {
    let clean_name =
        crate::services::scanner::core::normalizer::normalize_display_name(folder_name);
    let valid_name = if clean_name.is_empty() {
        folder_name.trim()
    } else {
        &clean_name
    };

    if target_enabled {
        return valid_name.to_string();
    }

    format!("DISABLED {valid_name}")
}

pub(crate) fn find_existing_sibling_case_insensitive(
    parent: &Path,
    target_name: &str,
    source_path: &Path,
) -> Option<PathBuf> {
    let entries = std::fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path == source_path {
            continue;
        }

        let entry_name = entry.file_name();
        if entry_name
            .to_string_lossy()
            .eq_ignore_ascii_case(target_name)
        {
            return Some(entry_path);
        }
    }

    None
}

pub(crate) fn rename_conflict_error(
    attempted_path: &Path,
    existing_path: &Path,
    base_name: &str,
) -> AppError {
    AppError::Io(
        serde_json::json!({
            "type": "RenameConflict",
            "message": "Target already exists",
            "attempted_target": attempted_path.to_string_lossy(),
            "existing_path": existing_path.to_string_lossy(),
            "base_name": base_name,
        })
        .to_string(),
    )
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
        let base = crate::services::scanner::core::normalizer::normalize_display_name(&old_name);
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

pub async fn toggle_mod_inner_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    state: &WatcherState,
    op_lock: &OperationLock,
    path: String,
    enable: bool,
    game_id: &str,
) -> Result<String, AppError> {
    toggle_mod_inner_service_with_duplicate_policy(
        config, pool, state, op_lock, path, enable, game_id, false,
    )
    .await
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
        PathGuard::validate_path(config, game_id, &path).map_err(AppError::Security)?;

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
        crate::database::models::ItemStatus::Enabled
    } else {
        crate::database::models::ItemStatus::Disabled
    };

    let disabled_reason = if enable {
        None
    } else {
        Some(crate::services::corridor_constants::DISABLED_REASON_USER)
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
    let is_safe: Option<i32> = sqlx::query_scalar(
        "SELECT is_safe FROM mods WHERE game_id = ? AND folder_path = ? LIMIT 1",
    )
    .bind(game_id)
    .bind(&new_rel)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(safe_val) = is_safe {
        let is_safe_bool = safe_val != 0;
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

    crate::services::runtime_projection_service::refresh_projection_for_object_ids(
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

    let new_folder_name =
        if crate::services::scanner::core::normalizer::is_disabled_folder(&old_folder_name) {
            format!("{}{}", crate::DISABLED_PREFIX, new_name)
        } else {
            new_name.clone()
        };

    let new_path = parent.join(&new_folder_name);
    if let Some(existing_path) =
        find_existing_sibling_case_insensitive(parent, &new_folder_name, path)
    {
        let base_name =
            crate::services::scanner::core::normalizer::normalize_display_name(&old_folder_name);
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
        PathGuard::validate_path(config, game_id, &old_path).map_err(AppError::Security)?;

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

    let is_safe: Option<i32> = sqlx::query_scalar(
        "SELECT is_safe FROM mods WHERE game_id = ? AND folder_path = ? LIMIT 1",
    )
    .bind(game_id)
    .bind(&new_rel)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(safe_val) = is_safe {
        let is_safe_bool = safe_val != 0;
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
