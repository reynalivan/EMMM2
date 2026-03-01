use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Simple struct for orphaned mods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanMod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPathInfo {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
}

// ── Single Operations ────────────────────────────────────────────────────────

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

pub async fn get_object_id_by_path(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_mod_by_id(pool: &SqlitePool, mod_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE id = ?")
        .bind(mod_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_mod_by_path(pool: &SqlitePool, folder_path: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE folder_path = ?")
        .bind(folder_path)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_mod_path_and_status(
    pool: &SqlitePool,
    game_id: &str,
    old_rel_path: &str,
    new_rel_path: &str,
    new_status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE mods SET folder_path = ?, status = ? WHERE folder_path = ? AND game_id = ?",
    )
    .bind(new_rel_path)
    .bind(new_status)
    .bind(old_rel_path)
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_mod_path_only(
    pool: &SqlitePool,
    game_id: &str,
    old_rel_path: &str,
    new_rel_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET folder_path = ? WHERE folder_path = ? AND game_id = ?")
        .bind(new_rel_path)
        .bind(old_rel_path)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_mod_path_by_old_path(
    pool: &SqlitePool,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET folder_path = ? WHERE folder_path = ?")
        .bind(new_path)
        .bind(old_path)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_child_paths(
    pool: &SqlitePool,
    game_id: &str,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<(), sqlx::Error> {
    let old_match = format!("{}%", old_prefix);
    sqlx::query(
        "UPDATE mods SET folder_path = ? || SUBSTR(folder_path, LENGTH(?) + 1) WHERE folder_path LIKE ? AND game_id = ?",
    )
    .bind(new_prefix)
    .bind(old_prefix)
    .bind(&old_match)
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_favorite_by_path(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    favorite: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET is_favorite = ? WHERE folder_path = ? AND game_id = ?")
        .bind(favorite)
        .bind(folder_path)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_pinned_by_path(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    pin: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path = ? AND game_id = ?")
        .bind(pin)
        .bind(folder_path)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Bulk Operations (Batched) ────────────────────────────────────────────────

pub async fn batch_update_path_and_status(
    pool: &SqlitePool,
    updates: &[(String, String, String)], // (old_path, new_path, new_status)
) -> Result<(), sqlx::Error> {
    if updates.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (old_path, new_path, new_status) in updates {
        sqlx::query("UPDATE mods SET folder_path = ?, status = ? WHERE folder_path = ?")
            .bind(new_path)
            .bind(new_status)
            .bind(old_path)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn batch_delete_by_path(pool: &SqlitePool, paths: &[String]) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("DELETE FROM mods WHERE folder_path = ?")
            .bind(path)
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

    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("UPDATE mods SET is_favorite = ? WHERE folder_path = ? AND game_id = ?")
            .bind(favorite)
            .bind(path)
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

    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path = ? AND game_id = ?")
            .bind(pin)
            .bind(path)
            .bind(game_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ── Read Metadata & Complex ──────────────────────────────────────────────────

pub async fn get_orphan_mods(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<OrphanMod>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, actual_name, folder_path FROM mods WHERE game_id = ? AND object_id IS NULL",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        result.push(OrphanMod {
            id: row.try_get("id")?,
            actual_name: row.try_get("actual_name").unwrap_or_default(),
            folder_path: row.try_get("folder_path")?,
        });
    }
    Ok(result)
}

pub async fn set_mod_object(
    pool: &SqlitePool,
    mod_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET object_id = ?, object_type = 'Other' WHERE id = ?")
        .bind(object_id)
        .bind(mod_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_disabled_mods_by_object_id(
    pool: &SqlitePool,
    object_id: &str,
    is_safe: bool,
) -> Result<Vec<ModPathInfo>, sqlx::Error> {
    let mut query = "SELECT id, actual_name, folder_path FROM mods WHERE object_id = ? AND status = 'DISABLED' AND folder_path NOT LIKE '%/.%' AND folder_path NOT LIKE '%\\.%'".to_string();
    if is_safe {
        query.push_str(" AND is_safe = 1");
    }

    let rows = sqlx::query(&query).bind(object_id).fetch_all(pool).await?;

    let mut result = Vec::new();
    for row in rows {
        result.push(ModPathInfo {
            id: row.try_get("id")?,
            actual_name: row.try_get("actual_name")?,
            folder_path: row.try_get("folder_path")?,
        });
    }
    Ok(result)
}

pub async fn get_enabled_mods_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'")
        .bind(game_id)
        .fetch_all(pool)
        .await?;

    let mut paths = Vec::new();
    for row in rows {
        paths.push(row.try_get("folder_path")?);
    }
    Ok(paths)
}

pub async fn get_object_id_by_folder_and_game(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_enabled_siblings_paths(
    pool: &SqlitePool,
    object_id: &str,
    game_id: &str,
    exclude_folder: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT folder_path FROM mods
         WHERE object_id = ? AND game_id = ? AND status = 'ENABLED'
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
         WHERE object_id = ? AND game_id = ? AND status = 'ENABLED'
         AND folder_path != ?",
    )
    .bind(object_id)
    .bind(game_id)
    .bind(exclude_folder)
    .fetch_all(pool)
    .await
}

pub async fn insert_new_mod(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    actual_name: &str,
    folder_path: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO mods (id, game_id, actual_name, folder_path, status, object_type, is_favorite, is_safe) VALUES (?, ?, ?, ?, ?, 'Other', 0, 1)"
    )
    .bind(id)
    .bind(game_id)
    .bind(actual_name)
    .bind(folder_path)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_mod_identity(
    pool: &SqlitePool,
    new_id: &str,
    new_folder_path: &str,
    new_actual_name: &str,
    new_status: &str,
    old_folder_path: &str,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET id = ?, folder_path = ?, actual_name = ?, status = ? WHERE folder_path = ? AND game_id = ?")
        .bind(new_id)
        .bind(new_folder_path)
        .bind(new_actual_name)
        .bind(new_status)
        .bind(old_folder_path)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_mod_by_path_and_game(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_enabled_mods_names_and_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT actual_name, folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn get_mods_with_uuid_format(
    pool: &SqlitePool,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, game_id, folder_path FROM mods WHERE length(id) = 36 AND id LIKE '%-%-%-%-%'",
    )
    .fetch_all(pool)
    .await
}

pub async fn update_mod_id(
    conn: &mut sqlx::SqliteConnection,
    old_id: &str,
    new_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET id = ? WHERE id = ?")
        .bind(new_id)
        .bind(old_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_mod_id_and_status_by_path(
    conn: &mut sqlx::SqliteConnection,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, object_id, status FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .fetch_optional(conn)
        .await
}

pub async fn update_mod_status_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_mod_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    game_id: &str,
    actual_name: &str,
    folder_path: &str,
    status: &str,
    object_type: &str,
    is_favorite: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_type, is_favorite) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(game_id)
    .bind(actual_name)
    .bind(folder_path)
    .bind(status)
    .bind(object_type)
    .bind(is_favorite)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn update_mod_object_id_and_type_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    object_id: &str,
    object_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
        .bind(object_id)
        .bind(object_type)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_all_mods_id_and_paths_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, folder_path FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(conn)
        .await
}

pub async fn delete_mod_tx(conn: &mut sqlx::SqliteConnection, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_mod_id_and_object_id_by_path(
    pool: &sqlx::SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    sqlx::query_as("SELECT id, object_id FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn snapshot_nsfw_mods_status_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET last_status_nsfw = (CASE WHEN status = 'ENABLED' THEN 1 ELSE 0 END) WHERE is_safe = 0")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn disable_all_nsfw_mods_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET status = 'DISABLED' WHERE is_safe = 0")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn restore_sfw_mods_status_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET status = 'ENABLED' WHERE is_safe = 1 AND last_status_sfw = 1")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn clear_sfw_last_status_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET last_status_sfw = 0 WHERE is_safe = 1 AND last_status_sfw = 1")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn snapshot_sfw_mods_status_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET last_status_sfw = (CASE WHEN status = 'ENABLED' THEN 1 ELSE 0 END) WHERE is_safe = 1")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn restore_nsfw_mods_status_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET status = 'ENABLED' WHERE is_safe = 0 AND last_status_nsfw = 1")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn clear_nsfw_last_status_tx(
    conn: &mut sqlx::SqliteConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET last_status_nsfw = 0 WHERE is_safe = 0 AND last_status_nsfw = 1")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_all_mods_id_path_status(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, folder_path, status FROM mods")
        .fetch_all(pool)
        .await
}

pub async fn update_mod_path_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
    new_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET folder_path = ? WHERE id = ?")
        .bind(new_path)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/mod_repo_test.rs"]
mod tests;
