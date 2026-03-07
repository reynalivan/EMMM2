use tauri::Manager;
use uuid::Uuid;

use crate::database::object_repo::{CreateObjectInput, UpdateObjectInput};
use crate::types::errors::CommandError;

pub async fn create_object_cmd_inner(
    pool: &sqlx::SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
    input: CreateObjectInput,
) -> Result<String, CommandError> {
    let id = Uuid::new_v4().to_string();
    let is_safe = input.is_safe.unwrap_or(true);
    let metadata_str = input
        .metadata
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let folder_path = input.folder_path.unwrap_or_else(|| input.name.clone());

    let mut thumbnail_abs_path: Option<String> = None;
    let mut pending_thumbnail_copy = None;
    let mut pending_folder_creation = None;

    // Lookup the game path (this is the mods root)
    let game_row = sqlx::query!("SELECT mod_path FROM games WHERE id = ?", input.game_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| CommandError::Database(e.to_string()))?;

    if let Some(game) = game_row {
        if let Some(mod_path) = game.mod_path {
            let target_dir = std::path::Path::new(&mod_path).join(&folder_path);

            pending_folder_creation = Some(target_dir.clone());

            // Determine if thumbnail_url is provided so we can derive the future dest path
            if let (Some(thumb), Some(app)) = (&input.thumbnail_url, app_handle) {
                if let Some(res_dir) = app.path().resource_dir().ok() {
                    let source_thumb: std::path::PathBuf = res_dir.join("databases").join(thumb);
                    if source_thumb.exists() {
                        let ext = source_thumb.extension().unwrap_or_default();
                        let dest_thumb =
                            target_dir.join(format!("preview.{}", ext.to_string_lossy()));

                        thumbnail_abs_path = Some(dest_thumb.to_string_lossy().to_string());
                        pending_thumbnail_copy = Some((source_thumb, dest_thumb));
                    }
                }
            }
        }
    }

    // 1. Insert to database FIRST to prevent race condition with filesystem watcher.
    // If the watcher triggers immediately after folder creation, it will find the DB record
    // and won't insert an "Other" category default placeholder.
    let res = crate::database::object_repo::create_object(
        pool,
        &id,
        &input.game_id,
        &input.name,
        &folder_path,
        &input.object_type,
        input.sub_category.as_ref(),
        is_safe,
        &metadata_str,
        thumbnail_abs_path.as_ref(),
    )
    .await;

    match res {
        Ok(_) => {
            // 2. Safely create folder now that the DB has the correct object logic
            if let Some(target_dir) = pending_folder_creation {
                if !target_dir.exists() {
                    let _ = std::fs::create_dir_all(&target_dir);
                }
            }

            // 3. Copy thumbnail if needed
            if let Some((src, dest)) = pending_thumbnail_copy {
                let _ = std::fs::copy(&src, &dest);
            }

            Ok(id)
        }
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique constraint failed") || msg.contains("idx_objects_game_name") {
                Err(CommandError::Database(format!(
                    "An object named '{}' already exists for this game.",
                    input.name.trim()
                )))
            } else {
                Err(e.into())
            }
        }
    }
}

/// Toggle the pinned state of an object.
pub async fn toggle_pin_object(pool: &sqlx::SqlitePool, id: &str, pin: bool) -> Result<(), String> {
    crate::database::object_repo::set_is_pinned(pool, id, pin)
        .await
        .map_err(|e| e.to_string())
}

/// Update an object, returning a user-friendly error on unique-name conflicts.
pub async fn update_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), CommandError> {
    match crate::database::object_repo::update_object(pool, id, updates).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique constraint failed") || msg.contains("idx_objects_game_name") {
                Err(CommandError::Database(
                    "An object with that name already exists.".to_string(),
                ))
            } else {
                Err(e.into())
            }
        }
    }
}
/// Delete an object: move its folder to trash, cascade-delete child mods, then remove the DB record.
pub async fn delete_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    trash_dir: &std::path::Path,
    watcher_state: &crate::services::scanner::watcher::WatcherState,
    op_lock: &crate::services::fs_utils::operation_lock::OperationLock,
) -> Result<(), CommandError> {
    // Acquire operation lock + suppress watcher for the entire operation
    let _lock = op_lock.acquire().await.map_err(CommandError::Io)?;
    let _guard =
        crate::services::scanner::watcher::SuppressionGuard::new(&watcher_state.suppressor);
    // 1. Fetch object from DB to get game_id and folder_path
    let row = sqlx::query!("SELECT game_id, folder_path FROM objects WHERE id = ?", id)
        .fetch_optional(pool)
        .await
        .map_err(|e| CommandError::Database(e.to_string()))?;

    let obj = row.ok_or_else(|| CommandError::NotFound(format!("Object not found: {}", id)))?;

    let mut target_dir_opt: Option<std::path::PathBuf> = None;
    let game_id = obj.game_id.clone();

    let game_row = sqlx::query!("SELECT mod_path FROM games WHERE id = ?", obj.game_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| CommandError::Database(e.to_string()))?;

    if let Some(game) = game_row {
        if let Some(mod_path) = game.mod_path {
            if let Some(ref folder_path) = obj.folder_path {
                target_dir_opt = Some(std::path::Path::new(&mod_path).join(folder_path));
            }
        }
    }

    // 2. Move folder to trash (if it exists on disk)
    if let Some(target_dir) = target_dir_opt {
        if target_dir.exists() {
            log::info!("delete_object: moving {:?} to trash", target_dir);
            crate::services::mods::trash::move_to_trash(
                &target_dir,
                trash_dir,
                Some(game_id.clone()),
            )
            .map_err(|e| {
                log::error!("delete_object: trash move failed: {}", e);
                CommandError::Io(format!(
                    "Failed to move folder '{}' to trash. {}",
                    target_dir.display(),
                    e
                ))
            })?;
            log::info!("delete_object: successfully trashed {:?}", target_dir);
        } else {
            log::info!(
                "delete_object: dir {:?} does not exist, skipping trash",
                target_dir
            );
        }
    } else {
        log::warn!(
            "delete_object: could not resolve folder path for object id={}",
            id
        );
    }

    // 3. Cascade-delete child mod rows from DB
    let deleted_mods = crate::database::object_repo::delete_mods_for_object(pool, id).await?;
    if deleted_mods > 0 {
        log::info!(
            "delete_object: cascade-deleted {} mod rows for object id={}",
            deleted_mods,
            id
        );
    }

    // 4. Delete the object record itself
    crate::database::object_repo::delete_object(pool, id).await?;
    log::info!("delete_object: removed object id={} from DB", id);
    Ok(())
}
