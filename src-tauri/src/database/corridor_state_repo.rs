use serde::Serialize;
use sqlx::SqlitePool;

/// Tracks which named collection is "active" in a corridor and which snapshot is available
/// for Undo. One row per (game_id, is_safe) pair.
#[derive(Debug, Clone, Serialize)]
pub struct CorridorState {
    /// Remembered collection context for corridor restore/switch flows.
    /// Strict current active state is resolved from runtime snapshot signatures.
    pub active_collection_id: Option<String>,
    /// UUID of the `is_last_unsaved` snapshot collection (for Undo).
    /// NULL means no Undo is available for this corridor.
    pub undo_collection_id: Option<String>,
}

/// Upserts the full corridor state. Both `active_collection_id` and `undo_collection_id`
/// are replaced atomically. Pass `None` to explicitly clear a pointer.
pub async fn upsert_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    active_collection_id: Option<&str>,
    undo_collection_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO corridor_state (game_id, is_safe, active_collection_id, undo_collection_id)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(game_id, is_safe) DO UPDATE SET
             active_collection_id = excluded.active_collection_id,
             undo_collection_id   = excluded.undo_collection_id",
    )
    .bind(game_id)
    .bind(is_safe)
    .bind(active_collection_id)
    .bind(undo_collection_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Updates only the remembered collection pointer, preserving the existing undo snapshot.
pub async fn update_active_collection_id(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    active_collection_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO corridor_state (game_id, is_safe, active_collection_id, undo_collection_id)
         VALUES (?, ?, ?, NULL)
         ON CONFLICT(game_id, is_safe) DO UPDATE SET
             active_collection_id = excluded.active_collection_id",
    )
    .bind(game_id)
    .bind(is_safe)
    .bind(active_collection_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns the corridor state for a given game/safe combination.
/// Always returns a value (never errors on missing row — returns all-None state instead).
pub async fn get_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CorridorState, sqlx::Error> {
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT cs.active_collection_id,
               cs.undo_collection_id
        FROM   corridor_state cs
        WHERE  cs.game_id = ? AND cs.is_safe = ?
        "#,
    )
    .bind(game_id)
    .bind(is_safe)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some((active_id, undo_id)) => CorridorState {
            active_collection_id: active_id,
            undo_collection_id: undo_id,
        },
        None => CorridorState {
            active_collection_id: None,
            undo_collection_id: None,
        },
    })
}
