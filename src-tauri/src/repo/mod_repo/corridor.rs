//! Safety corridor state (`mods.is_safe` / `mods.corridor_source`).
//! Safety is stored at mod level; objects are not safety-classified.

use super::paths::get_game_mod_path;
use crate::common::corridor_constants::CORRIDOR_SOURCE_MANUAL;
use crate::common::path_key::folder_path_key;
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
    Ok(value.flatten().map(|v| v != 0))
}

/// Distinct corridors (is_safe values) touched by the given relative folder paths.
pub async fn get_distinct_corridors_for_folders(
    pool: &SqlitePool,
    game_id: &str,
    folder_paths: &[String],
) -> Result<Vec<bool>, sqlx::Error> {
    let paths_json = serde_json::to_string(folder_paths).unwrap_or_else(|_| "[]".to_string());
    let rows: Vec<Option<i32>> = sqlx::query_scalar(
        "SELECT DISTINCT is_safe FROM mods WHERE game_id = ? AND folder_path IN (SELECT value FROM json_each(?))",
    )
    .bind(game_id)
    .bind(paths_json)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|v| v.unwrap_or(1) != 0).collect())
}

/// `is_safe` of a mod row, but only when the corridor was manually assigned.
pub async fn get_manual_is_safe_by_key(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path_key: &str,
) -> Result<Option<bool>, sqlx::Error> {
    let row: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT COALESCE(is_safe, 1), corridor_source FROM mods WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(game_id)
    .bind(folder_path_key)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.and_then(|(is_safe, corridor_source)| {
        (corridor_source.as_deref() == Some(CORRIDOR_SOURCE_MANUAL)).then_some(is_safe)
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

/// Update the safety classification of a mod by its relative folder_path.
/// Safety is stored at mod-level (`mods.is_safe`); objects are not safety-classified.
pub async fn set_mod_safe_by_path(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    safe: bool,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query(
        "UPDATE mods SET is_safe = ?, corridor_source = ? WHERE folder_path_key = ? AND game_id = ?",
    )
        .bind(safe)
        .bind(CORRIDOR_SOURCE_MANUAL)
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}
