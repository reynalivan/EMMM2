use tauri::Manager;
use uuid::Uuid;

use crate::shared::errors::AppError;
use crate::modules::catalog::domain::objects::{CreateObjectInput, UpdateObjectInput};

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
    let mut previous_thumbnail = None;

    let mods_path = crate::modules::games::adapters::outbound::sqlite::game::get_configured_mods_path(pool, &input.game_id)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Game mods path not configured".to_string()))?;
    let target_dir = std::path::Path::new(&mods_path).join(&folder_path);
    if let Some((attempted_path, existing_path, base_name)) = find_new_path_identity_conflict(
        std::path::Path::new(&mods_path),
        std::path::Path::new(&folder_path),
    ) {
        return Err(crate::modules::library::application::mods::core_ops::rename_conflict_error(
            &attempted_path,
            &existing_path,
            &base_name,
        ));
    }

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
        previous_thumbnail = Some(match std::fs::read(dest) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                cleanup_created_object_folder(&target_dir, created_folder);
                return Err(error.into());
            }
        });
        let thumbnail_bytes = std::fs::read(src).map_err(|error| {
            cleanup_created_object_folder(&target_dir, created_folder);
            AppError::Io(format!(
                "Failed to read object thumbnail '{}': {error}",
                src.display()
            ))
        })?;
        crate::platform::fs::atomic_file::atomic_write(dest, &thumbnail_bytes).map_err(
            |error| {
                cleanup_created_object_folder(&target_dir, created_folder);
                AppError::Io(format!(
                    "Failed to copy object thumbnail to '{}': {error}",
                    dest.display()
                ))
            },
        )?;
        crate::platform::images::thumbnail_cache::ThumbnailCache::invalidate(dest);
    }

    let res = crate::modules::catalog::adapters::outbound::sqlite::object::create_object(
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
            crate::modules::workspace::adapters::outbound::sqlite::runtime_projection::refresh_object_projection(
                pool,
                &input.game_id,
                &id,
            )
            .await
            .map_err(|e| AppError::Db(e.to_string()))?;

            Ok(id)
        }
        Err(e) => {
            if let Some((_, destination)) = pending_thumbnail_copy.as_ref() {
                let rollback = match previous_thumbnail.as_ref() {
                    Some(Some(bytes)) => {
                        crate::platform::fs::atomic_file::atomic_write(destination, bytes)
                    }
                    Some(None) if destination.exists() => {
                        std::fs::remove_file(destination).map_err(AppError::from)
                    }
                    _ => Ok(()),
                };
                if let Err(rollback_error) = rollback {
                    cleanup_created_object_folder(&target_dir, created_folder);
                    return Err(AppError::Io(format!(
                        "Object database insert failed ({e}); thumbnail rollback failed: {rollback_error}"
                    )));
                }
            }
            cleanup_created_object_folder(&target_dir, created_folder);
            if is_object_name_conflict(&e) {
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

/// SQLite names the objects(game_id, name) unique index differently across
/// versions; both spellings mean the same collision.
fn find_new_path_identity_conflict(
    root: &std::path::Path,
    relative_path: &std::path::Path,
) -> Option<(std::path::PathBuf, std::path::PathBuf, String)> {
    let mut parent = root.to_path_buf();
    for component in relative_path.components() {
        let std::path::Component::Normal(name) = component else {
            continue;
        };
        let target_name = name.to_string_lossy();
        let attempted_path = parent.join(name);
        let Some(existing_path) = crate::modules::library::application::mods::core_ops::find_sibling_identity_collision(
            &parent,
            &target_name,
            None,
        ) else {
            parent = attempted_path;
            continue;
        };
        let existing_name = existing_path.file_name()?.to_string_lossy();
        if existing_name.eq_ignore_ascii_case(&target_name) {
            parent = existing_path;
            continue;
        }
        return Some((
            attempted_path,
            existing_path,
            crate::modules::workspace::domain::normalizer::normalize_display_name(&target_name).into_owned(),
        ));
    }
    None
}

fn is_object_name_conflict(error: &sqlx::Error) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("unique constraint failed") || message.contains("idx_objects_game_name")
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

    // Anything but plain names — `..`, a root, a drive prefix — could escape the
    // mods tree once joined onto it.
    if !path
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(AppError::Validation(
            "Object folder path contains invalid components".to_string(),
        ));
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
    Ok(crate::modules::catalog::adapters::outbound::sqlite::object::set_is_pinned(pool, id, pin).await?)
}

/// Update an object, returning a user-friendly error on unique-name conflicts.
pub async fn update_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let object_game_id = crate::modules::catalog::adapters::outbound::sqlite::object::get_game_id_conn(&mut tx, id).await?;
    let update_result = async {
        crate::modules::catalog::adapters::outbound::sqlite::object::update_object(&mut *tx, id, updates).await?;
        if let Some(game_id) = object_game_id.as_deref() {
            crate::modules::workspace::adapters::outbound::sqlite::runtime_projection::refresh_projection_for_object_ids_tx(
                &mut tx,
                game_id,
                [id.to_string()],
            )
            .await?;
        }
        tx.commit().await
    }
    .await;

    match update_result {
        Ok(()) => Ok(()),
        Err(e) if is_object_name_conflict(&e) => Err(AppError::Db(
            "An object with that name already exists.".to_string(),
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn set_object_and_mods_category(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
    category: &str,
) -> Result<usize, AppError> {
    let category = category.trim();
    if category.is_empty() {
        return Err(AppError::Validation("Category is required".to_string()));
    }

    let mut tx = pool.begin().await?;
    let object_updated = crate::modules::catalog::adapters::outbound::sqlite::object::update_object_type_for_game(
        &mut *tx, game_id, object_id, category,
    )
    .await?;
    if object_updated == 0 {
        return Err(AppError::NotFound(format!(
            "Object '{object_id}' was not found for game '{game_id}'"
        )));
    }

    let child_updated =
        crate::modules::library::adapters::outbound::sqlite::mods::set_object_type_for_object(&mut *tx, game_id, object_id, category)
            .await?;
    crate::modules::workspace::adapters::outbound::sqlite::runtime_projection::refresh_projection_for_object_ids_tx(
        &mut tx,
        game_id,
        [object_id.to_string()],
    )
    .await?;
    tx.commit().await?;
    Ok(child_updated as usize)
}
/// Delete an object on disk. The command's trailing full reconcile is the
/// single writer that removes object/mod projections and records collection
/// members as missing before those runtime rows disappear.
pub async fn delete_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    force: bool,
    watcher_state: &crate::modules::workspace::application::scanner::watcher::WatcherState,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
) -> Result<(), AppError> {
    let _guard =
        crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(&watcher_state.suppressor);
    // 1. Fetch object from DB to get game_id and folder_path
    let (obj_game_id, obj_folder_path) =
        crate::modules::catalog::adapters::outbound::sqlite::object::get_game_id_and_folder_path(pool, id)
            .await
            .map_err(|e| AppError::Db(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {}", id)))?;

    let mut target_dir_opt: Option<std::path::PathBuf> = None;

    let mods_path = crate::modules::games::adapters::outbound::sqlite::game::get_configured_mods_path(pool, &obj_game_id)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;

    if let (Some(mods_path), Some(folder_path)) = (mods_path, obj_folder_path.as_ref()) {
        target_dir_opt = Some(std::path::Path::new(&mods_path).join(folder_path));
    }

    // 1.5. Safety Guard: Check if the object has any mods
    let count = crate::modules::catalog::adapters::outbound::sqlite::object::get_mod_count_for_object(pool, id).await?;
    if count > 0 && !force {
        return Err(AppError::ObjectHasMods(count as i32));
    }

    // 2. Move folder to trash (if it exists on disk)
    if let Some(target_dir) = target_dir_opt {
        if target_dir.exists() {
            log::info!("delete_object: moving {:?} to trash", target_dir);
            crate::modules::library::application::mods::trash::move_to_trash(&target_dir).map_err(|e| {
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

    Ok(())
}
