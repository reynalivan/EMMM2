//! Single-row updates: folder path, status, and user flags.

use super::paths::{get_game_mod_path, get_game_mod_path_for_mod_id};
use crate::common::path_key::{folder_path_key, strip_path_prefix_preserve_display};
use crate::domain::models::ItemStatus;
use sqlx::{Row, SqlitePool};

pub async fn update_mod_path_status_and_reason(
    pool: &SqlitePool,
    game_id: &str,
    old_rel_path: &str,
    new_rel_path: &str,
    new_status: ItemStatus,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query(
        "UPDATE mods
         SET folder_path = ?, folder_path_key = ?, status = ?, disabled_reason = ?
         WHERE folder_path_key = ? AND game_id = ?",
    )
    .bind(new_rel_path)
    .bind(folder_path_key(new_rel_path, mods_path.as_deref()))
    .bind(new_status as i64)
    .bind(reason)
    .bind(folder_path_key(old_rel_path, mods_path.as_deref()))
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_mod_path_by_old_path_in_game(
    pool: &SqlitePool,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query(
        "UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE folder_path_key = ? AND game_id = ?",
    )
        .bind(new_path)
        .bind(folder_path_key(new_path, mods_path.as_deref()))
        .bind(folder_path_key(old_path, mods_path.as_deref()))
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_child_paths(
    pool: &SqlitePool,
    game_id: &str,
    old_prefix: &str,
    new_prefix: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    // A transaction, not a bare connection: this rewrites one row per nested
    // mod, and a half-applied rewrite leaves the index pointing at paths that
    // no longer exist. It also collapses N autocommits into one.
    let mut tx = pool.begin().await?;
    update_child_paths_tx(&mut tx, game_id, old_prefix, new_prefix, mods_path).await?;
    tx.commit().await
}

/// Rewrites every mod path nested under `old_prefix` to sit under `new_prefix`.
/// Pass the bare folder paths: any trailing separator is stripped here and the
/// rebuilt paths always use `/`, so calling this once per separator only repeats
/// the same query.
pub async fn update_child_paths_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    old_prefix: &str,
    new_prefix: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    let old_root = old_prefix.trim_end_matches(['\\', '/']);
    let new_root = new_prefix.trim_end_matches(['\\', '/']);
    let old_path_key = folder_path_key(old_root, mods_path);
    let rows = sqlx::query(
        "SELECT id, folder_path FROM mods WHERE game_id = ? AND folder_path_key LIKE ?",
    )
    .bind(game_id)
    .bind(format!("{old_path_key}/%"))
    .fetch_all(&mut *conn)
    .await?;

    for row in rows {
        let id: String = row.try_get("id")?;
        let folder_path: String = row.try_get("folder_path")?;
        // `path_starts_with_key` is `strip_path_prefix_preserve_display(..).is_some()`,
        // so the `else` arm below already rejects non-matching rows.
        let Some(suffix) = strip_path_prefix_preserve_display(&folder_path, old_root, mods_path)
        else {
            continue;
        };
        if suffix.is_empty() {
            continue;
        }

        let new_path = format!("{new_root}/{suffix}");
        sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE id = ?")
            .bind(&new_path)
            .bind(folder_path_key(&new_path, mods_path))
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn update_mod_path_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
    new_path: &str,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path_for_mod_id(pool, id).await?;
    sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE id = ?")
        .bind(new_path)
        .bind(folder_path_key(new_path, mods_path.as_deref()))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_status_and_reason_for_object(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_folder: &str,
    new_status: ItemStatus,
    disabled_reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(&mut *conn, game_id).await?;
    let object_prefix_key = format!("{}/%", folder_path_key(object_folder, mods_path.as_deref()));
    sqlx::query(
        "UPDATE mods SET status = ?, disabled_reason = ? WHERE game_id = ? AND folder_path_key LIKE ?",
    )
    .bind(new_status as i64)
    .bind(disabled_reason)
    .bind(game_id)
    .bind(object_prefix_key)
    .execute(conn)
    .await?;
    Ok(())
}
