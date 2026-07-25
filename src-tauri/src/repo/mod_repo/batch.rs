//! Bulk operations, each committed in a single transaction.

use super::paths::get_game_mod_path;
use crate::common::path_key::folder_path_key;
use crate::domain::models::ItemStatus;
use sqlx::SqlitePool;

/// Paths MUST be absolute: the key is built without a `mods_path`, which only
/// resolves correctly because an absolute path short-circuits that lookup.
/// A relative path here would hash to a different key and silently match no rows.
pub async fn batch_update_path_and_status(
    pool: &SqlitePool,
    updates: &[(String, String, ItemStatus)], // (old_path, new_path, new_status)
) -> Result<(), sqlx::Error> {
    if updates.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (old_path, new_path, new_status) in updates {
        sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ?, status = ? WHERE folder_path_key = ?")
            .bind(new_path)
            .bind(folder_path_key(new_path, None))
            .bind(*new_status as i64)
            .bind(folder_path_key(old_path, None))
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Paths MUST be absolute — see [`batch_update_path_and_status`].
pub async fn batch_delete_by_path(pool: &SqlitePool, paths: &[String]) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("DELETE FROM mods WHERE folder_path_key = ?")
            .bind(folder_path_key(path, None))
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
