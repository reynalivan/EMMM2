//! Single-row lookups addressed by folder path or object id.

use super::paths::get_game_mod_path;
use crate::common::path_key::folder_path_key;
use sqlx::{Row, SqlitePool};

pub async fn get_mod_by_object_id(
    pool: &SqlitePool,
    object_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query("SELECT id, folder_path FROM mods WHERE object_id = ? LIMIT 1")
        .bind(object_id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        Ok(Some((r.try_get("id")?, r.try_get("folder_path")?)))
    } else {
        Ok(None)
    }
}

pub async fn get_object_id_by_folder_and_game(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path_key = ? AND game_id = ?")
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_mod_id_and_status_by_path(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>, i64)>, sqlx::Error> {
    get_mod_id_and_status_by_path_tx(&mut *pool.acquire().await?, folder_path, game_id).await
}

pub async fn get_mod_id_and_status_by_path_tx(
    conn: &mut sqlx::SqliteConnection,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>, i64)>, sqlx::Error> {
    let mods_path = get_game_mod_path(&mut *conn, game_id).await?;
    sqlx::query_as(
        "SELECT id, object_id, status FROM mods WHERE folder_path_key = ? AND game_id = ?",
    )
    .bind(folder_path_key(folder_path, mods_path.as_deref()))
    .bind(game_id)
    .fetch_optional(conn)
    .await
}

pub async fn get_mod_id_and_object_id_by_path(
    pool: &sqlx::SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query_as("SELECT id, object_id FROM mods WHERE folder_path_key = ? AND game_id = ?")
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_mod_id_by_path_tx(
    conn: &mut sqlx::SqliteConnection,
    folder_path: &str,
    game_id: &str,
    mods_path: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM mods WHERE folder_path_key = ? AND game_id = ?")
        .bind(folder_path_key(folder_path, mods_path))
        .bind(game_id)
        .fetch_optional(conn)
        .await
}
