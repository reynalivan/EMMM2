//! Multi-row reads: mod sets scoped by game or object.

use std::collections::HashMap;

use super::types::{Mod, ReconcileModRow};
use crate::modules::system::domain::mod_path::ModFolderPath;
use sqlx::SqlitePool;

fn is_effectively_enabled_path(folder_path: &str) -> bool {
    !folder_path
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .any(crate::modules::workspace::domain::normalizer::is_disabled_folder)
}

pub async fn get_rows_for_reconcile(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<ReconcileModRow>, sqlx::Error> {
    sqlx::query_as::<_, ReconcileModRow>(
        "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, safety_source, object_type, filesystem_identity FROM mods WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
}

#[derive(Clone, Copy)]
pub struct SafetyClassification {
    pub is_safe: bool,
    pub is_classified: bool,
}

/// Last-known safety classification keyed by the canonical relative mod path.
pub async fn get_safety_by_folder_path_key(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<HashMap<String, SafetyClassification>, sqlx::Error> {
    let rows: Vec<(String, bool, String)> = sqlx::query_as(
        "SELECT folder_path_key, COALESCE(is_safe, 1), COALESCE(safety_source, 'unknown') FROM mods WHERE game_id = ?",
    )
            .bind(game_id)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(key, is_safe, source)| {
            (
                key,
                SafetyClassification {
                    is_safe,
                    is_classified: source != crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN,
                },
            )
        })
        .collect())
}

pub async fn get_mods_by_object_id(
    pool: &SqlitePool,
    object_id: &str,
) -> Result<Vec<Mod>, sqlx::Error> {
    sqlx::query_as::<_, Mod>(
        "SELECT id, actual_name, folder_path, status FROM mods WHERE object_id = ?",
    )
    .bind(object_id)
    .fetch_all(pool)
    .await
}

pub async fn get_enabled_mods_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ModFolderPath>, sqlx::Error> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ? AND status = 1")
            .bind(game_id)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .filter(|path| is_effectively_enabled_path(path))
        .map(ModFolderPath::from_stored)
        .collect())
}

/// Canonical stored paths for every indexed terminal mod in one game.
pub async fn get_folder_paths_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ? ORDER BY folder_path_key")
        .bind(game_id)
        .fetch_all(pool)
        .await
}

pub async fn get_enabled_siblings_paths(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_folder: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT folder_path FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 1
         AND folder_path != ?",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_folder)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|path| is_effectively_enabled_path(path))
        .collect())
}

pub async fn get_enabled_duplicates(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_folder: &str,
) -> Result<Vec<(String, ModFolderPath, String)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, folder_path, actual_name FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 1
         AND folder_path != ?",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_folder)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, path, _)| is_effectively_enabled_path(path))
        .map(|(id, path, name)| (id, ModFolderPath::from_stored(path), name))
        .collect())
}

pub async fn get_enabled_mods_names_and_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, ModFolderPath)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT actual_name, folder_path FROM mods WHERE game_id = ? AND status = 1",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, path)| is_effectively_enabled_path(path))
        .map(|(name, path)| (name, ModFolderPath::from_stored(path)))
        .collect())
}

#[cfg(test)]
#[path = "tests/listing_tests.rs"]
mod tests;

pub async fn get_all_mods_id_and_paths_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<(String, ModFolderPath, bool)>, sqlx::Error> {
    let rows: Vec<(String, String, bool)> =
        sqlx::query_as("SELECT id, folder_path, COALESCE(is_safe, 1) FROM mods WHERE game_id = ?")
            .bind(game_id)
            .fetch_all(conn)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, path, is_safe)| (id, ModFolderPath::from_stored(path), is_safe))
        .collect())
}
