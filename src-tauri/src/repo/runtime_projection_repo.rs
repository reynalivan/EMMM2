//! Maintains the `object_runtime_projection` read-model table (pure SQL).
//!
//! This is a DB-only projection derived from `objects` + `mods`; it never
//! touches the filesystem. Writers of those tables (mutations, reconcile,
//! workspace switch) must call a refresh here after a successful change.

use sqlx::SqlitePool;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::common::safety_constants::SAFETY_SOURCE_UNKNOWN;

/// Counts only explicitly classified mods. Unknown mods remain visible in All
/// and are intentionally absent from both Safe and Unsafe filters.
fn classification_matches(is_safe: u8) -> String {
    format!(
        "COALESCE(m.is_safe, 1) = {is_safe}
            AND COALESCE(m.safety_source, '{SAFETY_SOURCE_UNKNOWN}') \
!= '{SAFETY_SOURCE_UNKNOWN}'"
    )
}

/// Built once: the classification predicate appeared six times as inline SQL with the
/// source names hardcoded, so a change to the rule had to land in six places.
static INSERT_PROJECTION_SQL: LazyLock<String> = LazyLock::new(|| {
    let safe_visible = classification_matches(1);
    let unsafe_visible = classification_matches(0);
    format!(
        r#"
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
            {safe_visible}
          )
    ) AS mod_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            {unsafe_visible}
          )
    ) AS mod_count_unsafe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            {safe_visible}
          )
    ) AS enabled_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            {unsafe_visible}
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
            {safe_visible}
          )
    ), '[]') AS active_mod_paths_safe_json,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            {unsafe_visible}
          )
    ), '[]') AS active_mod_paths_unsafe_json,
    CURRENT_TIMESTAMP
FROM objects o
WHERE o.game_id = ?
"#
    )
});

pub async fn rebuild_game_projection(pool: &SqlitePool, game_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    rebuild_game_projection_tx(&mut tx, game_id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn rebuild_game_projection_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM object_runtime_projection WHERE game_id = ?")
        .bind(game_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(&INSERT_PROJECTION_SQL)
        .bind(game_id)
        .execute(&mut *conn)
        .await?;
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

    // One transaction for the whole batch. Per-object commits made a cold
    // projection pay a commit per object, and left the read model observable
    // in a half-refreshed state between them.
    let mut tx = pool.begin().await?;
    for object_id in &unique_ids {
        refresh_object_projection_tx(&mut tx, game_id, object_id).await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn refresh_projection_for_object_ids_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_ids: impl IntoIterator<Item = String>,
) -> Result<(), sqlx::Error> {
    let unique_ids = object_ids
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<HashSet<_>>();
    for object_id in unique_ids {
        refresh_object_projection_tx(conn, game_id, &object_id).await?;
    }
    Ok(())
}

pub async fn refresh_object_projection(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    refresh_object_projection_tx(&mut tx, game_id, object_id).await?;
    tx.commit().await?;
    Ok(())
}

async fn refresh_object_projection_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM object_runtime_projection WHERE game_id = ? AND object_id = ?")
        .bind(game_id)
        .bind(object_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("{} AND o.id = ?", *INSERT_PROJECTION_SQL))
        .bind(game_id)
        .bind(object_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
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
