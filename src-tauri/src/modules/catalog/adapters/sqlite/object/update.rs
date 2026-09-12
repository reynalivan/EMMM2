use sqlx::{QueryBuilder, Sqlite};

use crate::modules::catalog::domain::objects::UpdateObjectInput;
use crate::modules::games::domain::models::ItemStatus;
use crate::shared::path_key::{canonical_name_key, folder_path_key};

pub async fn set_filesystem_identity_tx(
    conn: &mut sqlx::SqliteConnection,
    object_id: &str,
    filesystem_identity: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET filesystem_identity = ? WHERE id = ?")
        .bind(filesystem_identity)
        .bind(object_id)
        .execute(conn)
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

pub async fn update_object_type_for_game<'c, E>(
    executor: E,
    game_id: &str,
    object_id: &str,
    object_type: &str,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    let result = sqlx::query("UPDATE objects SET object_type = ? WHERE game_id = ? AND id = ?")
        .bind(object_type)
        .bind(game_id)
        .bind(object_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

pub async fn update_object_disk_identity_by_id<'c, E>(
    executor: E,
    object_id: &str,
    name: &str,
    folder_path: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET name = ?, name_key = ?, folder_path = ?, folder_path_key = ?, status = ?
         WHERE id = ?",
    )
    .bind(name)
    .bind(canonical_name_key(name))
    .bind(folder_path)
    .bind(folder_path_key(folder_path, None))
    .bind(status as i64)
    .bind(object_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn stage_object_identity_tx(
    conn: &mut sqlx::SqliteConnection,
    object_id: &str,
    stage_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE objects SET name = ?, name_key = ?, folder_path = ?, folder_path_key = ? WHERE id = ?",
    )
    .bind(stage_path)
    .bind(canonical_name_key(stage_path))
    .bind(stage_path)
    .bind(folder_path_key(stage_path, None))
    .bind(object_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// JSON sentinels persisted when a value fails to serialize.
const EMPTY_JSON_OBJECT: &str = "{}";
const EMPTY_JSON_ARRAY: &str = "[]";

pub async fn update_object<'c, E>(
    executor: E,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
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

    qb.build().execute(executor).await?;
    Ok(())
}
