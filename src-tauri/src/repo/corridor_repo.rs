use sqlx::{Row, SqliteConnection, SqlitePool};

use crate::domain::corridor::{CorridorRuntime, CorridorState};
use crate::domain::errors::CorridorError;

// ---------------------------------------------------------------------------
// corridor_repo — CRUD for `corridor_state` and `corridor_runtime_cache`
// ---------------------------------------------------------------------------

/// Get the corridor state (pointers) for a specific game + mode.
pub async fn get(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Option<CorridorState>, CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    let row = sqlx::query(
        r#"SELECT game_id, is_safe, active_collection_id, undo_collection_id
        FROM corridor_state
        WHERE game_id = ? AND is_safe = ?"#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| CorridorState {
        game_id: r.get("game_id"),
        is_safe: r.get::<i32, _>("is_safe") != 0,
        active_collection_id: r.get("active_collection_id"),
        undo_collection_id: r.get("undo_collection_id"),
    }))
}

/// Update the active/undo collection pointers.
pub async fn update_pointers(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    active_collection_id: Option<&str>,
    undo_collection_id: Option<&str>,
) -> Result<(), CorridorError> {
    let mut tx = pool.begin().await?;
    update_pointers_tx(
        &mut tx,
        game_id,
        is_safe,
        active_collection_id,
        undo_collection_id,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_pointers_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    is_safe: bool,
    active_collection_id: Option<&str>,
    undo_collection_id: Option<&str>,
) -> Result<(), CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    sqlx::query(
        r#"INSERT INTO corridor_state (game_id, is_safe, active_collection_id, undo_collection_id)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(game_id, is_safe) DO UPDATE SET
            active_collection_id = excluded.active_collection_id,
            undo_collection_id = excluded.undo_collection_id"#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .bind(active_collection_id)
    .bind(undo_collection_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Clear any stale active/undo pointers that reference a deleted collection.
pub async fn clear_collection_references_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(), CorridorError> {
    sqlx::query(
        r#"
        UPDATE corridor_state
        SET
            active_collection_id = CASE
                WHEN active_collection_id = ? THEN NULL
                ELSE active_collection_id
            END,
            undo_collection_id = CASE
                WHEN undo_collection_id = ? THEN NULL
                ELSE undo_collection_id
            END
        WHERE active_collection_id = ? OR undo_collection_id = ?
        "#,
    )
    .bind(collection_id)
    .bind(collection_id)
    .bind(collection_id)
    .bind(collection_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Upsert the runtime cache (physical state).
pub async fn upsert_runtime(
    pool: &SqlitePool,
    runtime: &CorridorRuntime,
) -> Result<(), CorridorError> {
    let is_safe_i32 = if runtime.is_safe { 1i32 } else { 0i32 };

    sqlx::query(
        r#"INSERT INTO corridor_runtime_cache 
           (game_id, is_safe, matched_collection_id, state_kind, state_name, signature, snapshot_json, snapshot_source, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(game_id, is_safe) DO UPDATE SET
            matched_collection_id = excluded.matched_collection_id,
            state_kind = excluded.state_kind,
            state_name = excluded.state_name,
            signature = excluded.signature,
            snapshot_json = excluded.snapshot_json,
            snapshot_source = excluded.snapshot_source,
            updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(&runtime.game_id)
    .bind(is_safe_i32)
    .bind(&runtime.matched_collection_id)
    .bind(&runtime.state_kind)
    .bind(&runtime.state_name)
    .bind(&runtime.signature)
    .bind(&runtime.snapshot_json)
    .bind(&runtime.snapshot_source)
    .execute(pool)
    .await?;

    Ok(())
}

/// Ensure a corridor row exists for a game + mode.
pub async fn ensure_exists(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<(), CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    sqlx::query(r#"INSERT OR IGNORE INTO corridor_state (game_id, is_safe) VALUES (?, ?)"#)
        .bind(game_id)
        .bind(is_safe_i32)
        .execute(pool)
        .await?;

    Ok(())
}

/// Update only the signature for a corridor.
pub async fn update_signature(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    signature: &str,
) -> Result<(), CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    sqlx::query(
        r#"INSERT INTO corridor_runtime_cache
           (game_id, is_safe, matched_collection_id, state_kind, state_name, signature, snapshot_json, snapshot_source, updated_at)
           VALUES (?, ?, NULL, 'unsaved', NULL, ?, ?, 'signature_update', CURRENT_TIMESTAMP)
           ON CONFLICT(game_id, is_safe) DO UPDATE SET
               signature = excluded.signature,
               updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .bind(signature)
    .bind(empty_projected_state_json())
    .execute(pool)
    .await?;

    Ok(())
}

fn empty_projected_state_json() -> &'static str {
    r#"{"object_states":[],"active_roots":[],"summary":{"object_count":0,"enabled_object_count":0,"active_root_count":0,"missing_root_count":0}}"#
}
