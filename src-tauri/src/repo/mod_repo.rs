use crate::database::models::ItemStatus;
use crate::services::corridor_constants::{
    CORRIDOR_SOURCE_AUTO_TAGGED, CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN,
    DISABLED_REASON_SYSTEM, DISABLED_REASON_USER,
};
use crate::services::path_key::{
    folder_path_key, path_starts_with_key, strip_path_prefix_preserve_display,
};
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
    pub status: ItemStatus,
}

async fn get_game_mod_path<'c, E>(executor: E, game_id: &str) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(executor)
        .await
        .map(|row| {
            row.and_then(|value| value.try_get("mods_path").ok())
                .flatten()
        })
}

async fn get_game_mod_path_for_mod_id<'c, E>(
    executor: E,
    mod_id: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "SELECT g.mods_path
         FROM mods m
         JOIN games g ON g.id = m.game_id
         WHERE m.id = ?",
    )
    .bind(mod_id)
    .fetch_optional(executor)
    .await
    .map(|row| {
        row.and_then(|value| value.try_get("mods_path").ok())
            .flatten()
    })
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
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query_scalar("SELECT object_id FROM mods WHERE folder_path_key = ? AND game_id = ?")
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
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
    sqlx::query("DELETE FROM mods WHERE folder_path_key = ?")
        .bind(folder_path_key(folder_path, None))
        .execute(pool)
        .await?;
    Ok(())
}

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

pub async fn update_mod_path_only(
    pool: &SqlitePool,
    game_id: &str,
    old_rel_path: &str,
    new_rel_path: &str,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE folder_path_key = ? AND game_id = ?")
        .bind(new_rel_path)
        .bind(folder_path_key(new_rel_path, mods_path.as_deref()))
        .bind(folder_path_key(old_rel_path, mods_path.as_deref()))
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
    sqlx::query("UPDATE mods SET folder_path = ?, folder_path_key = ? WHERE folder_path_key = ?")
        .bind(new_path)
        .bind(folder_path_key(new_path, None))
        .bind(folder_path_key(old_path, None))
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
    let old_root = old_prefix.trim_end_matches(['\\', '/']);
    let new_root = new_prefix.trim_end_matches(['\\', '/']);
    let old_path_key = folder_path_key(old_root, mods_path);
    let rows = sqlx::query(
        "SELECT id, folder_path FROM mods WHERE game_id = ? AND folder_path_key LIKE ?",
    )
    .bind(game_id)
    .bind(format!("{old_path_key}/%"))
    .fetch_all(pool)
    .await?;

    for row in rows {
        let id: String = row.try_get("id")?;
        let folder_path: String = row.try_get("folder_path")?;
        if !path_starts_with_key(&folder_path, old_root, mods_path) {
            continue;
        }

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
            .execute(pool)
            .await?;
    }
    Ok(())
}

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
        if !path_starts_with_key(&folder_path, old_root, mods_path) {
            continue;
        }

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

pub async fn update_status_for_object(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_folder: &str,
    new_status: ItemStatus,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(&mut *conn, game_id).await?;
    let object_prefix_key = format!("{}/%", folder_path_key(object_folder, mods_path.as_deref()));
    sqlx::query("UPDATE mods SET status = ? WHERE game_id = ? AND folder_path_key LIKE ?")
        .bind(new_status as i64)
        .bind(game_id)
        .bind(object_prefix_key)
        .execute(conn)
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

pub async fn set_favorite_by_path(
    pool: &SqlitePool,
    game_id: &str,
    folder_path: &str,
    favorite: bool,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query("UPDATE mods SET is_favorite = ? WHERE folder_path_key = ? AND game_id = ?")
        .bind(favorite)
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
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
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path_key = ? AND game_id = ?")
        .bind(pin)
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
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

// ── Bulk Operations (Batched) ────────────────────────────────────────────────

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

pub async fn batch_mark_system_disabled(
    pool: &SqlitePool,
    ids: &[String],
) -> Result<(), sqlx::Error> {
    if ids.is_empty() {
        return Ok(());
    }

    let mut qb: sqlx::QueryBuilder<'_, sqlx::Sqlite> =
        sqlx::QueryBuilder::new("UPDATE mods SET status = 0, disabled_reason = ");
    qb.push_bind(DISABLED_REASON_SYSTEM).push(" WHERE id IN (");
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(id);
    }
    sep.push_unseparated(")");
    qb.build().execute(pool).await?;
    Ok(())
}

pub async fn mark_enabled_clear_reason(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET status = 1, disabled_reason = NULL WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
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

pub async fn set_mod_object<'c, E>(
    executor: E,
    mod_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
        .bind(object_id)
        .bind(mod_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn get_disabled_mods_by_object_id(
    pool: &SqlitePool,
    object_id: &str,
    is_safe: bool,
) -> Result<Vec<ModPathInfo>, sqlx::Error> {
    let mut query = "SELECT m.id, m.actual_name, m.folder_path FROM mods m LEFT JOIN objects o ON m.object_id = o.id WHERE m.object_id = ? AND m.status = 0 AND m.folder_path NOT LIKE '%/.%' AND m.folder_path NOT LIKE '%\\.%'".to_string();
    if is_safe {
        query.push_str(" AND COALESCE(m.is_safe, 1) = 1");
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

pub async fn insert_new_mod<'c, E>(
    executor: E,
    id: &str,
    game_id: &str,
    object_id: &str,
    actual_name: &str,
    folder_path: &str,
    mods_path: Option<&str>,
    status: ItemStatus,
    is_safe: bool,
    corridor_source: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT OR IGNORE INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key, status, is_favorite, is_safe, corridor_source, size_bytes) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?, 0)"
    )
    .bind(id)
    .bind(game_id)
    .bind(object_id)
    .bind(actual_name)
    .bind(folder_path)
    .bind(folder_path_key(folder_path, mods_path.as_deref()))
    .bind(status as i64)
    .bind(is_safe)
    .bind(corridor_source)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_mod_identity<'c, E>(
    executor: E,
    new_id: &str,
    new_folder_path: &str,
    new_actual_name: &str,
    new_status: ItemStatus,
    old_id: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    let disabled_reason = if new_status.is_enabled() {
        None
    } else {
        Some(DISABLED_REASON_USER)
    };
    sqlx::query(
        "UPDATE mods
         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, disabled_reason = ?
         WHERE id = ?",
    )
        .bind(new_id)
        .bind(new_folder_path)
        .bind(folder_path_key(new_folder_path, mods_path.as_deref()))
        .bind(new_actual_name)
        .bind(new_status as i64)
        .bind(disabled_reason)
        .bind(old_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn update_mod_identity_tx(
    conn: &mut sqlx::SqliteConnection,
    new_id: &str,
    new_folder_path: &str,
    new_actual_name: &str,
    new_status: ItemStatus,
    new_is_safe: bool,
    corridor_source: &str,
    old_id: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    let disabled_reason = if new_status.is_enabled() {
        None
    } else {
        Some(DISABLED_REASON_USER)
    };
    sqlx::query(
        "UPDATE mods
         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, is_safe = ?, corridor_source = ?, disabled_reason = ?
         WHERE id = ?",
    )
        .bind(new_id)
        .bind(new_folder_path)
        .bind(folder_path_key(new_folder_path, mods_path.as_deref()))
        .bind(new_actual_name)
        .bind(new_status as i64)
        .bind(new_is_safe)
        .bind(corridor_source)
        .bind(disabled_reason)
        .bind(old_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_mod_by_path_and_game<'c, E>(
    executor: E,
    folder_path: &str,
    game_id: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("DELETE FROM mods WHERE folder_path_key = ? AND game_id = ?")
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
        .bind(game_id)
        .execute(executor)
        .await?;
    Ok(())
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

/// Pool-based variant for use outside transactions.
pub async fn get_mod_id_and_status_by_path_pool(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>, i64)>, sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query_as(
        "SELECT id, object_id, status FROM mods WHERE folder_path_key = ? AND game_id = ?",
    )
    .bind(folder_path_key(folder_path, mods_path.as_deref()))
    .bind(game_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_mod_status_and_reason_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    status: ItemStatus,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET status = ?, disabled_reason = ? WHERE id = ?")
        .bind(status as i64)
        .bind(reason)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn insert_mod_with_reason_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    game_id: &str,
    object_id: &str,
    actual_name: &str,
    folder_path: &str,
    mods_path: Option<&str>,
    status: ItemStatus,
    object_type: &str,
    is_favorite: bool,
    is_safe: bool,
    corridor_source: &str,
    disabled_reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key, status, object_type, is_favorite, is_safe, corridor_source, disabled_reason, size_bytes) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)"
    )
    .bind(id)
    .bind(game_id)
    .bind(object_id)
    .bind(actual_name)
    .bind(folder_path)
    .bind(folder_path_key(folder_path, mods_path.as_deref()))
    .bind(status as i64)
    .bind(object_type)
    .bind(is_favorite)
    .bind(is_safe)
    .bind(corridor_source)
    .bind(disabled_reason)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get_enabled_auto_tagged_mods_outside_corridor(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, folder_path
         FROM mods
         WHERE game_id = ?
           AND status = 1
           AND COALESCE(is_safe, 1) != ?
           AND COALESCE(corridor_source, ?) = ?",
    )
    .bind(game_id)
    .bind(is_safe)
    .bind(CORRIDOR_SOURCE_UNKNOWN)
    .bind(CORRIDOR_SOURCE_AUTO_TAGGED)
    .fetch_all(pool)
    .await
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
    let mods_path = get_game_mod_path(pool, game_id).await?;
    sqlx::query_as("SELECT id, object_id FROM mods WHERE folder_path_key = ? AND game_id = ?")
        .bind(folder_path_key(folder_path, mods_path.as_deref()))
        .bind(game_id)
        .fetch_optional(pool)
        .await
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

pub async fn get_system_disabled_mods(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_safe: bool,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, folder_path FROM mods WHERE game_id = ? AND status = 0 AND disabled_reason = ? AND COALESCE(is_safe, 1) = ?"
    )
    .bind(game_id)
    .bind(DISABLED_REASON_SYSTEM)
    .bind(target_safe)
    .fetch_all(pool)
    .await
}

/// Get a mapping of folder_path -> mod_id for all mods in a game.
/// Used for matching runtime mods to database identities.
pub async fn get_all_mods_mapping(
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
    let rows =
        sqlx::query_as::<_, (String, String)>("SELECT folder_path, id FROM mods WHERE game_id = ?")
            .bind(game_id)
            .fetch_all(pool)
            .await?;

    Ok(rows.into_iter().map(|(path, id)| (path, id)).collect())
}

pub async fn get_mod_id_and_status_by_path_any(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<(String, Option<String>, i64)>, sqlx::Error> {
    get_mod_id_and_status_by_path_pool(pool, folder_path, game_id).await
}

#[cfg(test)]
#[path = "tests/mod_repo_test.rs"]
mod tests;
