use uuid::Uuid;

use crate::database::object_repo::{CreateObjectInput, UpdateObjectInput};
use crate::types::errors::CommandError;

pub async fn create_object_cmd_inner(
    pool: &sqlx::SqlitePool,
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
    )
    .await;

    match res {
        Ok(_) => Ok(id),
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

/// Delete an object, guarding against non-empty objects.
pub async fn delete_object(pool: &sqlx::SqlitePool, id: &str) -> Result<(), CommandError> {
    let mod_count: i64 = crate::database::object_repo::get_mod_count_for_object(pool, id).await?;

    if mod_count > 0 {
        return Err(CommandError::Database(
            "Cannot delete object because it contains mods.".to_string(),
        ));
    }

    crate::database::object_repo::delete_object(pool, id).await?;
    Ok(())
}
