//! Maintains the `object_runtime_projection` read-model table (pure SQL).
//!
//! This is a DB-only projection derived from `objects` + `mods`; it never
//! touches the filesystem. Writers of those tables (mutations, reconcile,
//! workspace switch) must call a refresh here after a successful change.

use sqlx::SqlitePool;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::common::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};

/// A mod belongs to a corridor when it is classified into that corridor, or
/// when its classification is manual or unknown — those are visible in both.
///
/// `is_safe` is the corridor being counted: 1 for safe, 0 for unsafe.
fn corridor_visible(is_safe: u8) -> String {
    format!(
        "COALESCE(m.is_safe, 1) = {is_safe}
            OR COALESCE(m.corridor_source, '{CORRIDOR_SOURCE_UNKNOWN}') \
IN ('{CORRIDOR_SOURCE_MANUAL}', '{CORRIDOR_SOURCE_UNKNOWN}')"
    )
}

/// Built once: the corridor predicate appeared six times as inline SQL with the
/// source names hardcoded, so a change to the rule had to land in six places.
static INSERT_PROJECTION_SQL: LazyLock<String> = LazyLock::new(|| {
    let safe_visible = corridor_visible(1);
    let unsafe_visible = corridor_visible(0);
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
    sqlx::query("DELETE FROM object_runtime_projection WHERE game_id = ?")
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(&INSERT_PROJECTION_SQL)
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
