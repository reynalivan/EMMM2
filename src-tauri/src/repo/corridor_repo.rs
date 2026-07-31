use sqlx::{Row, SqliteConnection, SqlitePool};

use crate::domain::corridor::CorridorState;
use crate::domain::errors::CorridorError;

// ---------------------------------------------------------------------------
// corridor_repo — CRUD for `corridor_state`
// ---------------------------------------------------------------------------

/// Get the corridor state (pointers) for a specific game + mode.
pub async fn get(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Option<CorridorState>, CorridorError> {
    let row = sqlx::query(
        r#"SELECT game_id, is_safe, active_collection_id
        FROM corridor_state
        WHERE game_id = ? AND is_safe = ?"#,
    )
    .bind(game_id)
    .bind(is_safe)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| CorridorState {
        game_id: r.get("game_id"),
        is_safe: r.get::<i32, _>("is_safe") != 0,
        active_collection_id: r.get("active_collection_id"),
    }))
}

/// Update the active collection pointer.
pub async fn update_pointers_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    is_safe: bool,
    active_collection_id: Option<&str>,
) -> Result<(), CorridorError> {
    sqlx::query(
        r#"INSERT INTO corridor_state (game_id, is_safe, active_collection_id)
        VALUES (?, ?, ?)
        ON CONFLICT(game_id, is_safe) DO UPDATE SET
            active_collection_id = excluded.active_collection_id"#,
    )
    .bind(game_id)
    .bind(is_safe)
    .bind(active_collection_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Clear any stale active pointer that references a deleted collection.
pub async fn clear_collection_references_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(), CorridorError> {
    sqlx::query(
        "UPDATE corridor_state SET active_collection_id = NULL WHERE active_collection_id = ?",
    )
    .bind(collection_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Ensure a corridor row exists for a game + mode.
pub async fn ensure_exists(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<(), CorridorError> {
    sqlx::query(r#"INSERT OR IGNORE INTO corridor_state (game_id, is_safe) VALUES (?, ?)"#)
        .bind(game_id)
        .bind(is_safe)
        .execute(pool)
        .await?;

    Ok(())
}
