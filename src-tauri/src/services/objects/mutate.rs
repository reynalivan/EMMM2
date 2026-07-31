use tauri::Manager;
use uuid::Uuid;

use crate::domain::errors::AppError;
use crate::repo::object_repo::{CreateObjectInput, UpdateObjectInput};

pub async fn create_object_cmd_inner(
    pool: &sqlx::SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
    input: CreateObjectInput,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let metadata_str = input
        .metadata
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let folder_path = input.folder_path.unwrap_or_else(|| input.name.clone());
    validate_relative_object_folder(&folder_path)?;

    let mut thumbnail_abs_path: Option<String> = None;
    let mut pending_thumbnail_copy = None;

    let mods_path = crate::repo::game_repo::get_configured_mods_path(pool, &input.game_id)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Game mods path not configured".to_string()))?;
    let target_dir = std::path::Path::new(&mods_path).join(&folder_path);

    if let (Some(thumb), Some(app)) = (&input.thumbnail_url, app_handle) {
        if let Ok(res_dir) = app.path().resource_dir() {
            let source_thumb: std::path::PathBuf = res_dir.join("databases").join(thumb);
            if source_thumb.exists() {
                let ext = source_thumb.extension().unwrap_or_default();
                let dest_thumb = target_dir.join(format!("preview.{}", ext.to_string_lossy()));

                thumbnail_abs_path = Some(dest_thumb.to_string_lossy().to_string());
                pending_thumbnail_copy = Some((source_thumb, dest_thumb));
            }
        }
    }

    let created_folder = !target_dir.exists();
    std::fs::create_dir_all(&target_dir).map_err(|error| {
        AppError::Io(format!(
            "Failed to create object folder '{}': {error}",
            target_dir.display()
        ))
    })?;

    if !target_dir.is_dir() {
        return Err(AppError::Io(format!(
            "Failed to create object folder '{}': target is not a directory",
            target_dir.display()
        )));
    }

    if let Some((src, dest)) = &pending_thumbnail_copy {
        std::fs::copy(src, dest).map_err(|error| {
            cleanup_created_object_folder(&target_dir, created_folder);
            AppError::Io(format!(
                "Failed to copy object thumbnail to '{}': {error}",
                dest.display()
            ))
        })?;
    }

    let res = crate::repo::object_repo::create_object(
        pool,
        &id,
        &input.game_id,
        &input.name,
        &folder_path,
        &input.object_type,
        input.sub_category.as_ref(),
        input.status,
        &metadata_str,
        thumbnail_abs_path.as_ref(),
        None,
        None,
    )
    .await;

    match res {
        Ok(_) => {
            crate::repo::runtime_projection_repo::refresh_object_projection(
                pool,
                &input.game_id,
                &id,
            )
            .await
            .map_err(|e| AppError::Db(e.to_string()))?;

            Ok(id)
        }
        Err(e) => {
            cleanup_created_object_folder(&target_dir, created_folder);
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique constraint failed") || msg.contains("idx_objects_game_name") {
                Err(AppError::Db(format!(
                    "An object named '{}' already exists for this game.",
                    input.name.trim()
                )))
            } else {
                Err(e.into())
            }
        }
    }
}

fn validate_relative_object_folder(folder_path: &str) -> Result<(), AppError> {
    let trimmed = folder_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Object folder path cannot be empty".to_string(),
        ));
    }

    let path = std::path::Path::new(trimmed);
    if path.is_absolute() {
        return Err(AppError::Validation(
            "Object folder path must be relative".to_string(),
        ));
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(AppError::Validation(
                    "Object folder path contains invalid components".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn cleanup_created_object_folder(path: &std::path::Path, created_folder: bool) {
    if !created_folder {
        return;
    }

    if let Err(error) = std::fs::remove_dir(path) {
        log::warn!(
            "Failed to remove object folder '{}' after create failure: {}",
            path.display(),
            error
        );
    }
}

/// Toggle the pinned state of an object.
pub async fn toggle_pin_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    pin: bool,
) -> Result<(), AppError> {
    Ok(crate::repo::object_repo::set_is_pinned(pool, id, pin).await?)
}

/// Update an object, returning a user-friendly error on unique-name conflicts.
pub async fn update_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), AppError> {
    let object_game_id = crate::repo::object_repo::get_game_id(pool, id)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;

    match crate::repo::object_repo::update_object(pool, id, updates).await {
        Ok(_) => {
            if let Some(game_id) = object_game_id.as_deref() {
                crate::repo::runtime_projection_repo::refresh_object_projection(pool, game_id, id)
                    .await
                    .map_err(|e| AppError::Db(e.to_string()))?;
            }
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique constraint failed") || msg.contains("idx_objects_game_name") {
                Err(AppError::Db(
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
    force: bool,
    trash_dir: &std::path::Path,
    watcher_state: &crate::services::scanner::watcher::WatcherState,
    op_lock: &crate::services::fs_utils::operation_lock::OperationLock,
) -> Result<(), AppError> {
    // Acquire operation lock + suppress watcher for the entire operation
    let _lock = op_lock.acquire().await?;
    let _guard =
        crate::services::scanner::watcher::SuppressionGuard::new(&watcher_state.suppressor);
    // 1. Fetch object from DB to get game_id and folder_path
    let (obj_game_id, obj_folder_path) =
        crate::repo::object_repo::get_game_id_and_folder_path(pool, id)
            .await
            .map_err(|e| AppError::Db(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {}", id)))?;

    let mut target_dir_opt: Option<std::path::PathBuf> = None;

    let mods_path = crate::repo::game_repo::get_configured_mods_path(pool, &obj_game_id)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;

    if let (Some(mods_path), Some(folder_path)) = (mods_path, obj_folder_path.as_ref()) {
        target_dir_opt = Some(std::path::Path::new(&mods_path).join(folder_path));
    }

    // 1.5. Safety Guard: Check if the object has any mods
    let count = crate::repo::object_repo::get_mod_count_for_object(pool, id).await?;
    if count > 0 && !force {
        return Err(AppError::ObjectHasMods(count as i32));
    }

    // 2. Move folder to trash (if it exists on disk)
    if let Some(target_dir) = target_dir_opt {
        if target_dir.exists() {
            log::info!("delete_object: moving {:?} to trash", target_dir);
            crate::services::mods::trash::move_to_trash(
                &target_dir,
                trash_dir,
                Some(obj_game_id.clone()),
            )
            .map_err(|e| {
                log::error!("delete_object: trash move failed: {}", e);
                AppError::Io(format!(
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
    let deleted_mods = crate::repo::object_repo::delete_mods_for_object(pool, id).await?;
    if deleted_mods > 0 {
        log::info!(
            "delete_object: cascade-deleted {} mod rows for object id={}",
            deleted_mods,
            id
        );
    }

    // 4. Delete the object record itself
    crate::repo::object_repo::delete_object(pool, id).await?;
    crate::repo::runtime_projection_repo::delete_object_projection(pool, &obj_game_id, id)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;
    log::info!("delete_object: removed object id={} from DB", id);
    Ok(())
}
