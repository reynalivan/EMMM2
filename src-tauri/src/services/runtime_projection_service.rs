use sqlx::SqlitePool;
use std::collections::HashSet;

const INSERT_PROJECTION_SQL: &str = r#"
INSERT INTO object_runtime_projection (
    game_id,
    object_id,
    object_type,
    mod_count_safe,
    mod_count_unsafe,
    enabled_count_safe,
    enabled_count_unsafe,
    is_object_disabled,
    has_naming_conflict,
    active_mod_paths_safe_json,
    active_mod_paths_unsafe_json,
    updated_at
)
SELECT
    o.game_id,
    o.id,
    o.object_type,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS mod_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS mod_count_unsafe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS enabled_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS enabled_count_unsafe,
    CASE
        WHEN o.status = 0 THEN 1
        ELSE 0
    END AS is_object_disabled,
    0 AS has_naming_conflict,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ), '[]') AS active_mod_paths_safe_json,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ), '[]') AS active_mod_paths_unsafe_json,
    CURRENT_TIMESTAMP
FROM objects o
WHERE o.game_id = ?
"#;

pub async fn rebuild_game_projection(pool: &SqlitePool, game_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM object_runtime_projection WHERE game_id = ?")
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(INSERT_PROJECTION_SQL)
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Runtime projection is a DB read-model only.
/// Disk Reconcile owns filesystem truth, while Workspace Switch / DB-only mutations
/// must refresh projection explicitly after a successful state change.
pub async fn refresh_projection_for_object_ids(
    pool: &SqlitePool,
    game_id: &str,
    object_ids: &[String],
    fallback_full: bool,
) -> Result<(), sqlx::Error> {
    let unique_ids: Vec<String> = object_ids
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if unique_ids.is_empty() {
        if fallback_full {
            return rebuild_game_projection(pool, game_id).await;
        }

        return Ok(());
    }

    refresh_objects_projection(pool, game_id, &unique_ids).await
}

pub async fn refresh_object_projection(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM object_runtime_projection WHERE game_id = ? AND object_id = ?")
        .bind(game_id)
        .bind(object_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(&(String::from(INSERT_PROJECTION_SQL) + " AND o.id = ?"))
        .bind(game_id)
        .bind(object_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn refresh_objects_projection(
    pool: &SqlitePool,
    game_id: &str,
    object_ids: &[String],
) -> Result<(), sqlx::Error> {
    for object_id in object_ids {
        refresh_object_projection(pool, game_id, object_id).await?;
    }
    Ok(())
}

pub async fn refresh_paths_projection(
    pool: &SqlitePool,
    game_id: &str,
    folder_paths: &[String],
) -> Result<(), sqlx::Error> {
    use sqlx::Row;

    let mut object_ids = Vec::new();
    for folder_path in folder_paths {
        let rows = sqlx::query("SELECT id FROM objects WHERE game_id = ? AND folder_path = ?")
            .bind(game_id)
            .bind(folder_path)
            .fetch_all(pool)
            .await?;
        for row in rows {
            object_ids.push(row.try_get::<String, _>("id")?);
        }
    }
    refresh_objects_projection(pool, game_id, &object_ids).await
}

pub async fn delete_object_projection(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM object_runtime_projection WHERE game_id = ? AND object_id = ?")
        .bind(game_id)
        .bind(object_id)
        .execute(pool)
        .await?;
    Ok(())
}
