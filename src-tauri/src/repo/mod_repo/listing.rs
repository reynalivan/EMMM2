//! Multi-row reads: mod sets scoped by game or object.

use super::types::{Mod, ReconcileModRow};
use crate::domain::models::ItemStatus;
use sqlx::{Row, SqlitePool};

pub async fn get_rows_for_reconcile(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<ReconcileModRow>, sqlx::Error> {
    sqlx::query_as::<_, ReconcileModRow>(
        "SELECT id, folder_path, folder_path_key, actual_name, status, object_id, COALESCE(is_safe, 1) as is_safe, corridor_source, object_type FROM mods WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn get_mods_by_object_id(
    pool: &SqlitePool,
    object_id: &str,
    is_safe: bool,
) -> Result<Vec<Mod>, sqlx::Error> {
    let mut query =
        "SELECT id, actual_name, folder_path, status FROM mods WHERE object_id = ?".to_string();
    if is_safe {
        query.push_str(" AND COALESCE(is_safe, 1) = 1");
    }

    sqlx::query_as::<_, Mod>(&query)
        .bind(object_id)
        .fetch_all(pool)
        .await
}

pub async fn get_enabled_mods_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT folder_path FROM mods WHERE game_id = ? AND status = 1")
        .bind(game_id)
        .fetch_all(pool)
        .await?;

    let mut paths = Vec::new();
    for row in rows {
        paths.push(row.try_get("folder_path")?);
    }
    Ok(paths)
}

pub async fn get_enabled_siblings_paths(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_folder: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT folder_path FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 1
         AND folder_path != ?",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_folder)
    .fetch_all(pool)
    .await
}

pub async fn get_enabled_duplicates(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_folder: &str,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, folder_path, actual_name FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 1
         AND folder_path != ?",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_folder)
    .fetch_all(pool)
    .await
}

pub async fn get_enabled_mods_names_and_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT actual_name, folder_path FROM mods WHERE game_id = ? AND status = 1",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn get_all_mods_id_and_paths_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<(String, String, bool)>, sqlx::Error> {
    sqlx::query_as("SELECT id, folder_path, COALESCE(is_safe, 1) FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(conn)
        .await
}

pub async fn get_all_mods_sync_info_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<
    Vec<(
        String,
        String,
        ItemStatus,
        Option<String>,
        bool,
        Option<String>,
    )>,
    sqlx::Error,
> {
    sqlx::query_as(
        "SELECT id, folder_path, status, object_id, COALESCE(is_safe, 1), corridor_source FROM mods WHERE game_id = ?",
    )
        .bind(game_id)
        .fetch_all(conn)
        .await
}

/// Folder paths of every mod owned by any of `object_ids`, ordered by mod id.
pub async fn get_folder_paths_by_object_ids(
    pool: &SqlitePool,
    game_id: &str,
    object_ids: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    if object_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query_builder =
        sqlx::QueryBuilder::new("SELECT folder_path FROM mods WHERE game_id = ");
    query_builder.push_bind(game_id);
    query_builder.push(" AND object_id IN (");
    let mut separated = query_builder.separated(", ");
    for object_id in object_ids {
        separated.push_bind(object_id);
    }
    separated.push_unseparated(") ORDER BY id");

    query_builder
        .build_query_scalar::<String>()
        .fetch_all(pool)
        .await
}
