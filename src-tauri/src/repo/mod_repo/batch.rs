//! Bulk operations, each committed in a single transaction.

use super::paths::get_game_mod_path;
use crate::common::path_key::folder_path_key;
use crate::domain::models::ItemStatus;
use sqlx::SqlitePool;

/// Rewrite the path and status of each listed mod, within one game.
///
/// Paths are matched the way Disk Reconcile stores them: relative to the mods
/// root, keyed with no `mods_path`. Two games can hold the same mod at the
/// same relative path, so the key alone is ambiguous and `game_id` is what
/// makes the match single-game — its absence used to let a bulk toggle in one
/// game rewrite another game's row.
pub async fn batch_update_path_and_status(
    pool: &SqlitePool,
    game_id: &str,
    updates: &[(String, String, ItemStatus)], // (old_path, new_path, new_status)
) -> Result<(), sqlx::Error> {
    if updates.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (old_path, new_path, new_status) in updates {
        sqlx::query(
            "UPDATE mods SET folder_path = ?, folder_path_key = ?, status = ?
             WHERE folder_path_key = ? AND game_id = ?",
        )
        .bind(new_path)
        .bind(folder_path_key(new_path, None))
        .bind(*new_status as i64)
        .bind(folder_path_key(old_path, None))
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Delete the listed mods from one game — see [`batch_update_path_and_status`]
/// for why `game_id` is load-bearing rather than decorative.
pub async fn batch_delete_by_path(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("DELETE FROM mods WHERE folder_path_key = ? AND game_id = ?")
            .bind(folder_path_key(path, None))
            .bind(game_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn batch_set_favorite(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    favorite: bool,
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mods_path = get_game_mod_path(pool, game_id).await?;
    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("UPDATE mods SET is_favorite = ? WHERE folder_path_key = ? AND game_id = ?")
            .bind(favorite)
            .bind(folder_path_key(path, mods_path.as_deref()))
            .bind(game_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn batch_set_pinned(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    pin: bool,
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mods_path = get_game_mod_path(pool, game_id).await?;
    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path_key = ? AND game_id = ?")
            .bind(pin)
            .bind(folder_path_key(path, mods_path.as_deref()))
            .bind(game_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
