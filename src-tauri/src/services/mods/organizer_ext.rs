use crate::services::mods::bulk::{BulkActionError, BulkResult};
use crate::services::scanner::deep_matcher::MasterDb;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;

pub async fn auto_organize_mods_service(
    pool: &sqlx::SqlitePool,
    paths: Vec<String>,
    target_root: String,
    db_json: String,
    watcher: &WatcherState,
) -> Result<BulkResult, String> {
    use crate::services::scanner::core::organizer;

    let db = MasterDb::from_json(&db_json)?;
    let root = Path::new(&target_root);
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
                let _ = crate::database::mod_repo::update_mod_path_by_old_path(
                    pool, &path_str, &new_path,
                )
                .await;
                success.push(new_path);
            }
            Err(e) => {
                let error_str = match e {
                    organizer::OrganizeError::Duplicate { dest } => format!("DUPLICATE|{}", dest),
                    organizer::OrganizeError::Generic(msg) => msg,
                };
                failures.push(BulkActionError {
                    path: path_str,
                    error: error_str,
                });
            }
        }
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
/// 5. If `status == "only-enable"`, disable all sibling mods in the target.
pub async fn move_mod_to_object_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    folder_path: &str,
    target_object_id: &str,
    status: Option<&str>,
) -> Result<(), String> {
    use crate::services::scanner::core::normalizer::is_disabled_folder;

    // 1. Resolve paths from DB
    let game_mod_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Game not found".to_string())?;

    let target_obj_rel = crate::database::object_repo::get_folder_path(pool, target_object_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Target object not found".to_string())?;

    let base_path = Path::new(&game_mod_path);
    let target_obj_path = base_path.join(&target_obj_rel);

    if !target_obj_path.exists() {
        std::fs::create_dir_all(&target_obj_path).map_err(|e| e.to_string())?;
    }

    let current_path = Path::new(folder_path);
    if !current_path.exists() {
        return Err("Source mod folder does not exist".to_string());
    }

    let mod_folder_name = current_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    // 2. Determine ENABLED/DISABLED prefix based on desired status
    let is_currently_disabled = is_disabled_folder(&mod_folder_name);
    let mut new_mod_folder_name = mod_folder_name.clone();

    if let Some(status_val) = status {
        if status_val == "disabled" && !is_currently_disabled {
            new_mod_folder_name = standardize_prefix(&mod_folder_name, false);
        } else if status_val == "only-enable" && is_currently_disabled {
            new_mod_folder_name = standardize_prefix(&mod_folder_name, true);
        }
    }

    let new_path = target_obj_path.join(&new_mod_folder_name);

    // 3. Move (rename) the folder
    if current_path != new_path {
        std::fs::rename(current_path, &new_path).map_err(|e| e.to_string())?;
    }

    // 4. If "only-enable", disable all other mods in the target object directory
    if status == Some("only-enable") {
        if let Ok(entries) = std::fs::read_dir(&target_obj_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path != new_path {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !is_disabled_folder(&name) && !name.starts_with('.') {
                        let disabled_name = standardize_prefix(&name, false);
                        let disabled_path = target_obj_path.join(disabled_name);
                        let _ = std::fs::rename(&path, &disabled_path);
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
