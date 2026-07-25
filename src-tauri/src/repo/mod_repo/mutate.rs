//! Row lifecycle: inserts and deletes.

use crate::common::path_key::folder_path_key;
use crate::domain::models::ItemStatus;
use sqlx::SqlitePool;

#[allow(clippy::too_many_arguments)] // Repository insert keeps DB columns explicit at call sites.
pub async fn insert_new_mod<'c, E>(
    executor: E,
    id: &str,
    game_id: &str,
    object_id: &str,
    actual_name: &str,
    folder_path: &str,
    mods_path: Option<&str>,
    status: ItemStatus,
    is_safe: bool,
    corridor_source: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT OR IGNORE INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key, status, is_favorite, is_safe, corridor_source, size_bytes) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?, 0)"
    )
    .bind(id)
    .bind(game_id)
    .bind(object_id)
    .bind(actual_name)
    .bind(folder_path)
    .bind(folder_path_key(folder_path, mods_path))
    .bind(status as i64)
    .bind(is_safe)
    .bind(corridor_source)
    .execute(executor)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Repository insert keeps DB columns explicit at call sites.
pub async fn insert_mod_with_reason_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    game_id: &str,
    object_id: &str,
    actual_name: &str,
    folder_path: &str,
    mods_path: Option<&str>,
    status: ItemStatus,
    object_type: &str,
    is_favorite: bool,
    is_safe: bool,
    corridor_source: &str,
    disabled_reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key, status, object_type, is_favorite, is_safe, corridor_source, disabled_reason, size_bytes) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)"
    )
    .bind(id)
    .bind(game_id)
    .bind(object_id)
    .bind(actual_name)
    .bind(folder_path)
    .bind(folder_path_key(folder_path, mods_path))
    .bind(status as i64)
    .bind(object_type)
    .bind(is_favorite)
    .bind(is_safe)
    .bind(corridor_source)
    .bind(disabled_reason)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn delete_mod_by_id(pool: &SqlitePool, mod_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE id = ?")
        .bind(mod_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// `folder_path` MUST be absolute: the key is built without a `mods_path`, and
/// only an absolute path short-circuits that lookup to the stored key shape.
pub async fn delete_mod_by_path(pool: &SqlitePool, folder_path: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE folder_path_key = ?")
        .bind(folder_path_key(folder_path, None))
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_mod_tx(conn: &mut sqlx::SqliteConnection, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
