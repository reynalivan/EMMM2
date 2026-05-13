use std::path::Path;

use crate::database::models::ItemStatus;
use crate::domain::errors::AppError;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::core_ops::{
    find_existing_sibling_case_insensitive, rename_conflict_error, standardize_prefix,
};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

pub struct ObjectSwitchOutcome {
    pub object_id: String,
    pub original_path: String,
    pub next_path: String,
}

fn build_object_path_candidates(
    mods_path: &Path,
    stored_folder_path: &str,
    object_name: &str,
) -> Vec<String> {
    let mut candidates = Vec::new();
    let stored_path = Path::new(stored_folder_path);
    if stored_path.is_absolute() {
        candidates.push(stored_path.to_string_lossy().to_string());
    } else {
        candidates.push(
            mods_path
                .join(stored_folder_path)
                .to_string_lossy()
                .to_string(),
        );
    }

    candidates.push(mods_path.join(object_name).to_string_lossy().to_string());
    candidates.push(
        mods_path
            .join(format!("{}{}", crate::DISABLED_PREFIX, object_name))
            .to_string_lossy()
            .to_string(),
    );

    candidates
}

fn find_matching_object_root(mods_path: &Path, object_name: &str) -> Option<String> {
    let expected_key = crate::services::path_key::canonical_name_key(object_name);
    let entries = std::fs::read_dir(mods_path).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(folder_name) = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
        else {
            continue;
        };

        let folder_key = crate::services::path_key::canonical_name_key(&folder_name);
        if folder_key == expected_key {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

async fn heal_object_root_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    old_folder_path: &str,
    new_folder_path: &str,
    mods_path: &str,
) -> Result<(), AppError> {
    if old_folder_path == new_folder_path {
        return Ok(());
    }

    crate::repo::object_repo::update_object_runtime_folder_path(
        pool,
        game_id,
        old_folder_path,
        new_folder_path,
    )
    .await?;

    for (old_sep, new_sep) in [
        (
            format!("{old_folder_path}\\"),
            format!("{new_folder_path}\\"),
        ),
        (format!("{old_folder_path}/"), format!("{new_folder_path}/")),
    ] {
        crate::repo::mod_repo::update_child_paths(
            pool,
            game_id,
            &old_sep,
            &new_sep,
            Some(mods_path),
        )
        .await?;
    }

    let mut tx = pool.begin().await?;
    crate::services::collection_service::handle_object_renamed_tx(
        &mut tx,
        old_folder_path,
        new_folder_path,
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    tx.commit().await?;

    Ok(())
}

fn map_toggle_error(path: &Path, source_path: &str, error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        let processes = crate::services::fs_utils::locking::get_locking_processes(path);
        if !processes.is_empty() {
            return AppError::FileInUse {
                path: source_path.to_string(),
                processes,
            };
        }

        return AppError::PathBusy {
            path: source_path.to_string(),
        };
    }

    AppError::Io(format!("Failed to rename object folder: {error}"))
}

async fn resolve_object_root_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<
    (
        crate::services::scanner::core::types::GameObject,
        String,
        String,
    ),
    AppError,
> {
    let object = crate::repo::object_repo::get_game_object_by_id(pool, object_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Object not found: {object_id}")))?;
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);

    for candidate in build_object_path_candidates(mods_root, &object.folder_path, &object.name) {
        if Path::new(&candidate).exists() {
            let relative_candidate = Path::new(&candidate)
                .strip_prefix(mods_root)
                .ok()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or(candidate.clone());
            heal_object_root_path(
                pool,
                game_id,
                &object.folder_path,
                &relative_candidate,
                &mods_path,
            )
            .await?;
            return Ok((object, mods_path, candidate));
        }
    }

    if let Some(found_path) = find_matching_object_root(mods_root, &object.name) {
        let relative_candidate = Path::new(&found_path)
            .strip_prefix(mods_root)
            .ok()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or(found_path.clone());
        heal_object_root_path(
            pool,
            game_id,
            &object.folder_path,
            &relative_candidate,
            &mods_path,
        )
        .await?;
        return Ok((object, mods_path, found_path));
    }

    Err(AppError::RuntimePathNotFound {
        target: object.name.clone(),
    })
}

/// Workspace Switch owns explicit object-root enable/disable.
/// Do not route object targets through mod-toggle services or Disk Reconcile.
pub async fn toggle_object_root_service(
    config: &crate::services::config::ConfigService,
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    op_lock: &OperationLock,
    game_id: &str,
    object_id: &str,
    enable: bool,
) -> Result<ObjectSwitchOutcome, AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Io)?;
    let _guard = SuppressionGuard::new(&watcher_state.suppressor);

    let (_object, mods_path, current_absolute_path) =
        resolve_object_root_path(pool, game_id, object_id).await?;
    let current_path = Path::new(&current_absolute_path);
    if !current_path.exists() || !current_path.is_dir() {
        return Err(AppError::RuntimePathNotFound {
            target: current_absolute_path,
        });
    }

    let old_name = current_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let new_name = standardize_prefix(&old_name, enable);
    if new_name == old_name {
        crate::services::runtime_projection_service::refresh_projection_for_object_ids(
            pool,
            game_id,
            &[object_id.to_string()],
            false,
        )
        .await?;
        return Ok(ObjectSwitchOutcome {
            object_id: object_id.to_string(),
            original_path: current_absolute_path.clone(),
            next_path: current_absolute_path,
        });
    }

    let parent = current_path
        .parent()
        .ok_or_else(|| AppError::Io("Invalid object root path".to_string()))?;
    let next_absolute_path = parent.join(&new_name);
    if let Some(existing_path) =
        find_existing_sibling_case_insensitive(parent, &new_name, current_path)
    {
        let base_name =
            crate::services::scanner::core::normalizer::normalize_display_name(&old_name);
        return Err(rename_conflict_error(
            &next_absolute_path,
            &existing_path,
            &base_name,
        ));
    }

    crate::services::fs_utils::file_utils::rename_cross_drive_fallback(
        current_path,
        &next_absolute_path,
    )
    .map_err(|error| map_toggle_error(current_path, &current_absolute_path, error))?;

    let mods_root = Path::new(&mods_path);
    let old_relative_path = current_path
        .strip_prefix(mods_root)
        .unwrap_or(current_path)
        .to_string_lossy()
        .to_string();
    let new_relative_path = next_absolute_path
        .strip_prefix(mods_root)
        .unwrap_or(&next_absolute_path)
        .to_string_lossy()
        .to_string();

    let mut tx = pool.begin().await?;
    crate::repo::object_repo::update_object_runtime_folder_path(
        &mut *tx,
        game_id,
        &old_relative_path,
        &new_relative_path,
    )
    .await?;
    crate::repo::mod_repo::update_child_paths_tx(
        &mut tx,
        game_id,
        &format!("{old_relative_path}\\"),
        &format!("{new_relative_path}\\"),
        Some(&mods_path),
    )
    .await?;
    crate::repo::mod_repo::update_child_paths_tx(
        &mut tx,
        game_id,
        &format!("{old_relative_path}/"),
        &format!("{new_relative_path}/"),
        Some(&mods_path),
    )
    .await?;
    crate::repo::mod_repo::update_status_and_reason_for_object(
        &mut tx,
        game_id,
        &new_relative_path,
        if enable {
            ItemStatus::Enabled
        } else {
            ItemStatus::Disabled
        },
        if enable {
            None
        } else {
            Some(crate::services::corridor_constants::DISABLED_REASON_USER)
        },
    )
    .await?;
    crate::repo::object_repo::update_object_status(
        &mut *tx,
        object_id,
        if enable {
            ItemStatus::Enabled
        } else {
            ItemStatus::Disabled
        },
    )
    .await?;
    crate::services::collection_service::handle_object_renamed_tx(
        &mut tx,
        &old_relative_path,
        &new_relative_path,
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    tx.commit().await?;

    crate::services::runtime_projection_service::refresh_projection_for_object_ids(
        pool,
        game_id,
        &[object_id.to_string()],
        false,
    )
    .await?;
    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        pool,
        config,
        watcher_state.suppressor.clone(),
        game_id,
        &[true, false],
        true,
        true,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(ObjectSwitchOutcome {
        object_id: object_id.to_string(),
        original_path: current_absolute_path,
        next_path: next_absolute_path.to_string_lossy().to_string(),
    })
}
