use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::types::*;
use crate::common::path_key::{canonical_name_key, folder_path_key};
use crate::domain::models::ItemStatus;

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

pub async fn update_object(
    pool: &SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE objects SET ");
    let mut is_first = true;

    if let Some(name) = &updates.name {
        if !is_first {
            qb.push(", ");
        }
        qb.push("name = ");
        qb.push_bind(name.trim().to_string());
        qb.push(", name_key = ");
        qb.push_bind(canonical_name_key(name));
        is_first = false;
    }
    if let Some(obj_type) = &updates.object_type {
        if !is_first {
            qb.push(", ");
        }
        qb.push("object_type = ");
        qb.push_bind(obj_type);
        is_first = false;
    }
    if let Some(st) = &updates.status {
        if !is_first {
            qb.push(", ");
        }
        qb.push("status = ");
        qb.push_bind(*st as i64);
        is_first = false;
    }
    if let Some(sub) = &updates.sub_category {
        if !is_first {
            qb.push(", ");
        }
        qb.push("sub_category = ");
        qb.push_bind(sub);
        is_first = false;
    }
    if let Some(meta) = &updates.metadata {
        if !is_first {
            qb.push(", ");
        }
        qb.push("metadata = ");
        qb.push_bind(meta.to_string());
        is_first = false;
    }
    if let Some(hash) = &updates.hash_db {
        if !is_first {
            qb.push(", ");
        }
        qb.push("hash_db = ");
        qb.push_bind(serde_json::to_string(hash).unwrap_or_else(|_| "{}".to_string()));
        is_first = false;
    }
    if let Some(skins) = &updates.custom_skins {
        if !is_first {
            qb.push(", ");
        }
        qb.push("custom_skins = ");
        qb.push_bind(serde_json::to_string(skins).unwrap_or_else(|_| "{}".to_string()));
        is_first = false;
    }
    if let Some(thumb) = &updates.thumbnail_path {
        if !is_first {
            qb.push(", ");
        }
        qb.push("thumbnail_path = ");
        qb.push_bind(thumb);
        is_first = false;
    }
    if let Some(auto) = updates.is_auto_sync {
        if !is_first {
            qb.push(", ");
        }
        qb.push("is_auto_sync = ");
        qb.push_bind(auto);
        is_first = false;
    }
    if let Some(pinned) = updates.is_pinned {
        if !is_first {
            qb.push(", ");
        }
        qb.push("is_pinned = ");
        qb.push_bind(pinned);
        is_first = false;
    }

    if let Some(tags) = &updates.tags {
        if !is_first {
            qb.push(", ");
        }
        qb.push("tags = ");
        qb.push_bind(serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string()));
        is_first = false;
    }

    if is_first {
        return Ok(());
    }

    qb.push(" WHERE id = ");
    qb.push_bind(id);

    qb.build().execute(pool).await?;
    Ok(())
}
