use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::bulk::{BulkActionError, BulkResult};
use crate::services::scanner::deep_matcher::MasterDb;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;

pub async fn auto_organize_mods_service(
    _config: &ConfigService,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    paths: Vec<String>,
    db_json: &str,
    watcher: &WatcherState,
) -> Result<BulkResult, AppError> {
    use crate::services::scanner::core::organizer;

    let db = MasterDb::from_json(db_json).map_err(AppError::Io)?;
    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let root = Path::new(&mods_path);
    let mut success = Vec::new();
    let mut failures = Vec::new();

    for path_str in paths {
        let path = Path::new(&path_str);

        // Suppress watcher
        let _guard = SuppressionGuard::new(&watcher.suppressor);

        match organizer::organize_mod(path, root, &db) {
            Ok(res) => {
                let new_path = res.new_path.to_string_lossy().to_string();
                // Sync DB: update folder_path to new location
                let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
                    pool, &path_str, &new_path, None,
                )
                .await;
                success.push(new_path);
            }
            Err(e) => {
                let error_str = match e {
                    organizer::OrganizeError::Collision(info) => {
                        format!(
                            "COLLISION|{}",
                            serde_json::to_string(&info).unwrap_or_default()
                        )
                    }
                    organizer::OrganizeError::Generic(msg) => msg,
                };
                failures.push(BulkActionError {
                    path: path_str,
                    error: AppError::Io(error_str),
                });
            }
        }
    }

    if !success.is_empty() {
        let _ = crate::services::collection_service::handle_dirty_state(pool, game_id, true).await;
        let _ = crate::services::collection_service::handle_dirty_state(pool, game_id, false).await;
    }

    Ok(BulkResult { success, failures })
}

/// Move a mod folder to a different object directory, optionally changing its enabled/disabled status.
///
/// Steps:
/// 1. Resolve game mod_path and target object relative path from DB.
/// 2. Create the target directory if missing.
/// 3. Optionally adjust enabled/disabled folder prefix.
/// 4. Rename (move) the folder.
/// 5. Update the DB `mods` table (path, status, object_id).
/// 6. If `status == "only-enable"`, disable all sibling mods in the target.
pub async fn move_mod_to_object_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    op_lock: &OperationLock,
    watcher: &WatcherState,
    game_id: &str,
    folder_path: &str,
    target_object_id: &str,
    status: Option<&str>,
) -> Result<(), AppError> {
    use crate::database::models::ItemStatus;
    use crate::services::scanner::core::normalizer::is_disabled_folder;

    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);

    let current_path =
        PathGuard::validate_path(config, game_id, folder_path).map_err(AppError::Security)?;

    // 1. Resolve paths from DB
    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;

    let target_obj = crate::repo::object_repo::get_game_object_by_id(pool, target_object_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Target object not found".to_string()))?;

    if target_obj.game_id != game_id {
        return Err(AppError::Validation(format!(
            "Target object '{}' belongs to game '{}', but requested move is for game '{}'",
            target_object_id, target_obj.game_id, game_id
        )));
    }

    let target_obj_rel = target_obj.folder_path;

    let base_path = Path::new(&game_mod_path);
    let target_obj_path = base_path.join(&target_obj_rel);

    if !target_obj_path.exists() {
        std::fs::create_dir_all(&target_obj_path).map_err(|e| AppError::Io(e.to_string()))?;
    }

    let mod_folder_name = current_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    // 2. Determine ENABLED/DISABLED prefix and status
    let is_currently_disabled = is_disabled_folder(&mod_folder_name);
    let mut new_mod_folder_name = mod_folder_name.clone();
    let mut new_status = if is_currently_disabled {
        ItemStatus::Disabled
    } else {
        ItemStatus::Enabled
    };

    if let Some(status_val) = status {
        if status_val == "disabled" {
            new_mod_folder_name = standardize_prefix(&mod_folder_name, false);
            new_status = ItemStatus::Disabled;
        } else if status_val == "only-enable" {
            new_mod_folder_name = standardize_prefix(&mod_folder_name, true);
            new_status = ItemStatus::Enabled;
        }
    }

    let new_path = target_obj_path.join(&new_mod_folder_name);
    let old_rel = current_path
        .strip_prefix(base_path)
        .unwrap_or(&current_path)
        .to_string_lossy()
        .to_string();
    let new_rel = new_path
        .strip_prefix(base_path)
        .unwrap_or(&new_path)
        .to_string_lossy()
        .to_string();

    // 3. Move (rename) the folder
    if current_path != new_path {
        std::fs::rename(&current_path, &new_path).map_err(|e| AppError::Io(e.to_string()))?;
    }

    // 4. Update DB Core Tables (mods)
    // We update the path, status, and the new object_id link.
    // metadata.rs doesn't have a direct helper for all 3, so we use repo calls.
    {
        // Update object connection
        let mod_id_status =
            crate::repo::mod_repo::get_mod_id_and_status_by_path_any(pool, &old_rel, game_id)
                .await?;
        if let Some((mod_id, _, _)) = mod_id_status {
            crate::repo::mod_repo::set_mod_object(pool, &mod_id, target_object_id).await?;
        }

        // Update path and status
        crate::repo::mod_repo::update_mod_path_status_and_reason(
            pool,
            game_id,
            &old_rel,
            &new_rel,
            new_status,
            if new_status == ItemStatus::Disabled {
                Some("User Disabled")
            } else {
                None
            },
        )
        .await?;

        // Inform collections
        let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
            pool,
            &old_rel,
            &new_rel,
            Some(target_object_id),
        )
        .await;
    }

    // 5. If "only-enable", disable all other mods in the target object directory
    if status == Some("only-enable") {
        if let Ok(entries) = std::fs::read_dir(&target_obj_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path != new_path {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !is_disabled_folder(&name) && !name.starts_with('.') {
                        let disabled_name = standardize_prefix(&name, false);
                        let disabled_path = target_obj_path.join(&disabled_name);
                        let sib_old_rel = path
                            .strip_prefix(base_path)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();
                        let sib_new_rel = disabled_path
                            .strip_prefix(base_path)
                            .unwrap_or(&disabled_path)
                            .to_string_lossy()
                            .to_string();

                        if std::fs::rename(&path, &disabled_path).is_ok() {
                            // Update DB status for sibling
                            let _ = crate::repo::mod_repo::update_mod_path_status_and_reason(
                                pool,
                                game_id,
                                &sib_old_rel,
                                &sib_new_rel,
                                ItemStatus::Disabled,
                                Some("Collision (Only-One-Active)"),
                            )
                            .await;

                            let _ =
                                crate::services::collection_service::handle_mod_moved_or_renamed(
                                    pool,
                                    &sib_old_rel,
                                    &sib_new_rel,
                                    None,
                                )
                                .await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Normalize a folder name's DISABLED prefix.
fn standardize_prefix(name: &str, enable: bool) -> String {
    use crate::services::scanner::core::normalizer::normalize_display_name;
    let base = normalize_display_name(name);
    if enable {
        base
    } else {
        format!("{}{}", crate::DISABLED_PREFIX, base)
    }
}
