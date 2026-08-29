use sqlx::SqlitePool;

use crate::common::path_key::{canonical_name_key, folder_path_key};
use crate::domain::models::ItemStatus;

#[allow(clippy::too_many_arguments)] // Repository insert keeps DB columns explicit at call sites.
pub async fn create_object(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    folder_path: &str,
    object_type: &str,
    sub_category: Option<&String>,
    status: Option<ItemStatus>,
    metadata_str: &str,
    thumbnail_path: Option<&String>,
    hash_db_str: Option<&str>,
    custom_skins_str: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, status, object_type, sub_category, is_auto_sync, tags, metadata, hash_db, custom_skins, thumbnail_path, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, '[]', ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(id)
    .bind(game_id)
    .bind(name)
    .bind(canonical_name_key(name))
    .bind(folder_path)
    .bind(folder_path_key(folder_path, None))
    .bind(status.unwrap_or(ItemStatus::Enabled) as i64) // DEFAULT ENABLED (1)
    .bind(object_type)
    .bind(sub_category)
    .bind(metadata_str)
    .bind(hash_db_str)
    .bind(custom_skins_str)
    .bind(thumbnail_path)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_object(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM objects WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Atomically delete an object folder and all its child mods from the DB.
///
/// Used when the watcher detects a depth=1 `Removed` event (an entire object
/// folder was deleted from disk). The operation runs inside a single transaction:
/// 1. Delete all `mods` rows whose `folder_path` starts with `{folder_path}/` or `{folder_path}\`
/// 2. Delete the `objects` row with `folder_path = folder_path AND game_id = game_id`
///
/// Idempotent — safe to call even if the object does not exist.
pub async fn delete_object_and_mods_by_folder(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
) -> Result<u64, sqlx::Error> {
    let mods_path: Option<String> = sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(&mut *conn)
        .await?
        .flatten();
    let child_prefix_key = format!("{}/%", folder_path_key(folder_path, mods_path.as_deref()));
    let mods_deleted = sqlx::query("DELETE FROM mods WHERE game_id = ? AND folder_path_key LIKE ?")
        .bind(game_id)
        .bind(child_prefix_key)
        .execute(&mut *conn)
        .await?
        .rows_affected();

    // Delete the object itself
    sqlx::query("DELETE FROM objects WHERE game_id = ? AND folder_path_key = ?")
        .bind(game_id)
        .bind(folder_path_key(folder_path, None))
        .execute(&mut *conn)
        .await?;

    log::info!(
        "delete_object_and_mods_by_folder: removed object folder='{}' game='{}', {} child mods deleted",
        folder_path, game_id, mods_deleted
    );
    Ok(mods_deleted)
}

/// Delete all mod rows belonging to an object (cascade helper).
pub async fn delete_mods_for_object(
    pool: &SqlitePool,
    object_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM mods WHERE object_id = ?")
        .bind(object_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn delete_ghost_objects_gc(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM objects WHERE game_id = $1 AND NOT EXISTS (SELECT 1 FROM mods WHERE object_id = objects.id)"
    )
    .bind(game_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_is_pinned(
    pool: &SqlitePool,
    id: &str,
    is_pinned: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET is_pinned = ? WHERE id = ?")
        .bind(is_pinned)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
