use sqlx::{Row, SqlitePool};

use crate::domain::corridor::{CorridorRuntime, CorridorSnapshot, CorridorState};
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

/// Get the runtime cache (physical state) for a corridor.
pub async fn get_runtime(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Option<CorridorRuntime>, CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    let row = sqlx::query(
        r#"SELECT game_id, is_safe, matched_collection_id, state_kind, state_name,
                  signature, snapshot_json, snapshot_source, updated_at
        FROM corridor_runtime_cache
        WHERE game_id = ? AND is_safe = ?"#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| CorridorRuntime {
        game_id: r.get("game_id"),
        is_safe: r.get::<i32, _>("is_safe") != 0,
        matched_collection_id: r.get("matched_collection_id"),
        state_kind: r.get("state_kind"),
        state_name: r.get("state_name"),
        signature: r.get("signature"),
        snapshot_json: r.get("snapshot_json"),
        snapshot_source: r.get("snapshot_source"),
        updated_at: r.get("updated_at"),
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
    .execute(pool)
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

/// Build a frontend-ready corridor snapshot.
pub async fn get_snapshot(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CorridorSnapshot, CorridorError> {
    let state = get(pool, game_id, is_safe).await?;
    let runtime = get_runtime(pool, game_id, is_safe).await?;

    let active_collection_name: Option<String> = if let Some(ref s) = state {
        if let Some(ref id) = s.active_collection_id {
            sqlx::query_scalar("SELECT name FROM collections WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?
        } else {
            None
        }
    } else {
        None
    };

    Ok(CorridorSnapshot {
        game_id: game_id.to_string(),
        is_safe,
        active_collection_id: state.as_ref().and_then(|s| s.active_collection_id.clone()),
        active_collection_name,
        undo_collection_id: state.as_ref().and_then(|s| s.undo_collection_id.clone()),
        current_signature: runtime
            .as_ref()
            .map(|r| r.signature.clone())
            .unwrap_or_default(),
        is_dirty: runtime.as_ref().is_some_and(|r| r.state_kind == "unsaved"),
    })
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
        r#"UPDATE corridor_runtime_cache SET signature = ?, updated_at = CURRENT_TIMESTAMP 
           WHERE game_id = ? AND is_safe = ?"#,
    )
    .bind(signature)
    .bind(game_id)
    .bind(is_safe_i32)
    .execute(pool)
    .await?;

    Ok(())
}

/// Record a switch event (update timestamp).
pub async fn record_switch(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<(), CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    sqlx::query(
        r#"UPDATE corridor_runtime_cache SET updated_at = CURRENT_TIMESTAMP 
           WHERE game_id = ? AND is_safe = ?"#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .execute(pool)
    .await?;

    Ok(())
}
