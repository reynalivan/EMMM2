//! Mod safety classification and its provenance.

use super::paths::get_game_mod_path;
use crate::common::path_key::folder_path_key;
use crate::common::safety_constants::SAFETY_SOURCE_MANUAL;
use sqlx::SqlitePool;

pub async fn get_is_safe_by_folder(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<Option<bool>, sqlx::Error> {
    let value: Option<Option<i32>> = sqlx::query_scalar(
        "SELECT is_safe FROM mods WHERE game_id = ? AND folder_path = ? LIMIT 1",
    )
    .bind(game_id)
    .bind(folder_path)
    .fetch_optional(pool)
    .await?;
    Ok(value.flatten().map(|value| value != 0))
}

pub async fn get_manual_is_safe_by_key(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path_key: &str,
) -> Result<Option<bool>, sqlx::Error> {
    let row: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT COALESCE(is_safe, 1), safety_source FROM mods WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(game_id)
    .bind(folder_path_key)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.and_then(|(is_safe, safety_source)| {
        (safety_source.as_deref() == Some(SAFETY_SOURCE_MANUAL)).then_some(is_safe)
    }))
}

pub async fn count_active_unsafe_mods(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM mods WHERE game_id = ? AND status = 1 AND COALESCE(is_safe, 1) = 0",
    )
    .bind(game_id)
    .fetch_one(pool)
    .await
}

pub async fn set_mod_safe_by_path(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    safe: bool,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query(
        "UPDATE mods SET is_safe = ?, safety_source = ? WHERE folder_path_key = ? AND game_id = ?",
    )
    .bind(safe)
    .bind(SAFETY_SOURCE_MANUAL)
    .bind(folder_path_key(folder_path, mods_path.as_deref()))
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
}
