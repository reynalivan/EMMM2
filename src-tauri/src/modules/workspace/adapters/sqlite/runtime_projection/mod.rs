//! Maintains the `object_runtime_projection` read-model table (pure SQL).
//!
//! This is a DB-only projection derived from `objects` + `mods`; it never
//! touches the filesystem. Writers of those tables (mutations, reconcile,
//! workspace switch) must call a refresh here after a successful change.

use sqlx::SqlitePool;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN;

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

#[cfg(test)]
mod tests {
    use super::refresh_projection_for_object_ids_tx;
    use sqlx::{Connection, SqliteConnection};
    use std::fs;
    use std::time::{Duration, Instant};

    const OBJECT_COUNT: usize = 1_000;
    const SAMPLE_COUNT: usize = 5;

    fn median<T: Ord + Copy>(mut samples: Vec<T>) -> T {
        samples.sort_unstable();
        samples[samples.len() / 2]
    }

    async fn total_changes(conn: &mut SqliteConnection) -> i64 {
        sqlx::query_scalar("SELECT total_changes()")
            .fetch_one(conn)
            .await
            .expect("SQLite should report total changes")
    }

    async fn refresh(
        conn: &mut SqliteConnection,
        object_ids: &[String],
    ) -> Result<(), sqlx::Error> {
        let mut tx = conn.begin().await?;
        refresh_projection_for_object_ids_tx(&mut *tx, "game", object_ids.to_vec()).await?;
        tx.commit().await
    }

    #[tokio::test]
    #[ignore = "manual P2 runtime projection I/O baseline"]
    async fn benchmark_noop_projection_refresh_1000_objects() {
        let workspace = tempfile::tempdir().expect("temporary workspace should be created");
        let database_path = workspace.path().join("projection.sqlite");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&database_path)
            .create_if_missing(true);
        let mut conn = SqliteConnection::connect_with(&options)
            .await
            .expect("temporary SQLite database should open");
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&mut conn)
            .await
            .expect("WAL mode should be enabled");
        sqlx::query("PRAGMA wal_autocheckpoint = 0")
            .execute(&mut conn)
            .await
            .expect("automatic checkpoint should be disabled for measurement");
        sqlx::query(
            "CREATE TABLE objects (id TEXT PRIMARY KEY, game_id TEXT, object_type TEXT, status INTEGER)",
        )
        .execute(&mut conn)
        .await
        .expect("objects table should be created");
        sqlx::query(
            "CREATE TABLE mods (object_id TEXT, is_safe INTEGER, safety_source TEXT, status INTEGER, folder_path TEXT)",
        )
        .execute(&mut conn)
        .await
        .expect("mods table should be created");
        sqlx::query(
            "CREATE TABLE object_runtime_projection (game_id TEXT, object_id TEXT, object_type TEXT, mod_count_safe INTEGER, mod_count_unsafe INTEGER, enabled_count_safe INTEGER, enabled_count_unsafe INTEGER, is_object_disabled INTEGER, has_naming_conflict INTEGER, active_mod_paths_safe_json TEXT, active_mod_paths_unsafe_json TEXT, updated_at TEXT)",
        )
        .execute(&mut conn)
        .await
        .expect("projection table should be created");

        let object_ids: Vec<String> = (0..OBJECT_COUNT)
            .map(|index| format!("object-{index}"))
            .collect();
        for object_id in &object_ids {
            sqlx::query(
                "INSERT INTO objects (id, game_id, object_type, status) VALUES (?, 'game', 'Character', 1)",
            )
            .bind(object_id)
            .execute(&mut conn)
            .await
            .expect("fixture object should be inserted");
        }

        refresh(&mut conn, &object_ids)
            .await
            .expect("initial projection should be built");
        let wal_path = database_path.with_extension("sqlite-wal");
        let mut elapsed = Vec::<Duration>::with_capacity(SAMPLE_COUNT);
        let mut row_changes = Vec::<i64>::with_capacity(SAMPLE_COUNT);
        let mut wal_growth = Vec::<u64>::with_capacity(SAMPLE_COUNT);

        for _ in 0..SAMPLE_COUNT {
            let changes_before = total_changes(&mut conn).await;
            let wal_before = fs::metadata(&wal_path)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            let started = Instant::now();
            refresh(&mut conn, &object_ids)
                .await
                .expect("no-op projection refresh should succeed");
            elapsed.push(started.elapsed());
            row_changes.push(total_changes(&mut conn).await - changes_before);
            let wal_after = fs::metadata(&wal_path)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            wal_growth.push(wal_after.saturating_sub(wal_before));
        }

        assert!(row_changes
            .iter()
            .all(|changes| *changes == (OBJECT_COUNT * 2) as i64));
        eprintln!(
            "p2_projection_noop_1000: p50={:?}; max={:?}; sqlite_row_changes_per_refresh={}; wal_growth_p50={} bytes",
            median(elapsed.clone()),
            elapsed.iter().copied().max().expect("samples should exist"),
            median(row_changes),
            median(wal_growth),
        );
    }
}
