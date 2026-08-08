use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::common::path_key::{canonical_name_key, folder_path_key};
use crate::domain::models::ItemStatus;
use crate::domain::objects::UpdateObjectInput;

pub async fn update_object_folder_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?,
             name = CASE WHEN name = ? THEN ? ELSE name END,
             name_key = CASE WHEN name = ? THEN ? ELSE name_key END
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(new_path)
    .bind(folder_path_key(new_path, None))
    .bind(old_path)
    .bind(new_path)
    .bind(old_path)
    .bind(canonical_name_key(new_path))
    .bind(game_id)
    .bind(folder_path_key(old_path, None))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_runtime_folder_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(new_path)
    .bind(folder_path_key(new_path, None))
    .bind(game_id)
    .bind(folder_path_key(old_path, None))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_runtime_state_by_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?,
             status = ?
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(new_path)
    .bind(folder_path_key(new_path, None))
    .bind(status as i64)
    .bind(game_id)
    .bind(folder_path_key(old_path, None))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_runtime_state_by_id<'c, E>(
    executor: E,
    object_id: &str,
    folder_path: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?,
             status = ?
         WHERE id = ?",
    )
    .bind(folder_path)
    .bind(folder_path_key(folder_path, None))
    .bind(status as i64)
    .bind(object_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_status<'c, E>(
    executor: E,
    object_id: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("UPDATE objects SET status = ? WHERE id = ?")
        .bind(status as i64)
        .bind(object_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// JSON sentinels persisted when a value fails to serialize.
const EMPTY_JSON_OBJECT: &str = "{}";
const EMPTY_JSON_ARRAY: &str = "[]";

pub async fn update_object(
    pool: &SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE objects SET ");
    // `separated` owns the comma, so each field is one line and cannot get the
    // punctuation wrong; `wrote_any` is all that is left of the old `is_first`
    // flag that every branch had to remember to clear.
    let mut sets = qb.separated(", ");
    let mut wrote_any = false;

    if let Some(name) = &updates.name {
        sets.push("name = ")
            .push_bind_unseparated(name.trim().to_string());
        sets.push("name_key = ")
            .push_bind_unseparated(canonical_name_key(name));
        wrote_any = true;
    }
    if let Some(obj_type) = &updates.object_type {
        sets.push("object_type = ").push_bind_unseparated(obj_type);
        wrote_any = true;
    }
    if let Some(st) = &updates.status {
        sets.push("status = ").push_bind_unseparated(*st as i64);
        wrote_any = true;
    }
    if let Some(sub) = &updates.sub_category {
        sets.push("sub_category = ").push_bind_unseparated(sub);
        wrote_any = true;
    }
    if let Some(meta) = &updates.metadata {
        sets.push("metadata = ")
            .push_bind_unseparated(meta.to_string());
        wrote_any = true;
    }
    if let Some(hash) = &updates.hash_db {
        sets.push("hash_db = ").push_bind_unseparated(
            serde_json::to_string(hash).unwrap_or_else(|_| EMPTY_JSON_OBJECT.to_string()),
        );
        wrote_any = true;
    }
    if let Some(skins) = &updates.custom_skins {
        sets.push("custom_skins = ").push_bind_unseparated(
            serde_json::to_string(skins).unwrap_or_else(|_| EMPTY_JSON_OBJECT.to_string()),
        );
        wrote_any = true;
    }
    if let Some(thumb) = &updates.thumbnail_path {
        sets.push("thumbnail_path = ").push_bind_unseparated(thumb);
        wrote_any = true;
    }
    if let Some(auto) = updates.is_auto_sync {
        sets.push("is_auto_sync = ").push_bind_unseparated(auto);
        wrote_any = true;
    }
    if let Some(pinned) = updates.is_pinned {
        sets.push("is_pinned = ").push_bind_unseparated(pinned);
        wrote_any = true;
    }
    if let Some(tags) = &updates.tags {
        sets.push("tags = ").push_bind_unseparated(
            serde_json::to_string(tags).unwrap_or_else(|_| EMPTY_JSON_ARRAY.to_string()),
        );
        wrote_any = true;
    }

    if !wrote_any {
        return Ok(());
    }

    qb.push(" WHERE id = ");
    qb.push_bind(id);

    qb.build().execute(pool).await?;
    Ok(())
}
