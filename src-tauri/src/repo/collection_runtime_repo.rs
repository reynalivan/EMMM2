use sqlx::{Row, SqliteConnection, SqlitePool};

#[derive(Debug, Clone, Default)]
pub struct CollectionRuntimeState {
    pub game_id: String,
    pub active_collection_id: Option<String>,
    pub draft_collection_id: Option<String>,
    pub draft_base_collection_id: Option<String>,
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> CollectionRuntimeState {
    CollectionRuntimeState {
        game_id: row.get("game_id"),
        active_collection_id: row.get("active_collection_id"),
        draft_collection_id: row.get("draft_collection_id"),
        draft_base_collection_id: row.get("draft_base_collection_id"),
    }
}

pub async fn get(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<CollectionRuntimeState>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT game_id, active_collection_id, draft_collection_id, draft_base_collection_id FROM collection_runtime_state WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(map_row))
}

pub async fn get_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Option<CollectionRuntimeState>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT game_id, active_collection_id, draft_collection_id, draft_base_collection_id FROM collection_runtime_state WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.map(map_row))
}

pub async fn set_active_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    collection_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO collection_runtime_state (game_id, active_collection_id)
           VALUES (?, ?)
           ON CONFLICT(game_id) DO UPDATE SET
             active_collection_id = excluded.active_collection_id,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(collection_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_active(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    set_active_tx(&mut tx, game_id, collection_id).await?;
    tx.commit().await
}

pub async fn set_draft_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    draft_collection_id: &str,
    base_collection_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO collection_runtime_state
             (game_id, draft_collection_id, draft_base_collection_id)
           VALUES (?, ?, ?)
           ON CONFLICT(game_id) DO UPDATE SET
             draft_collection_id = excluded.draft_collection_id,
             draft_base_collection_id = excluded.draft_base_collection_id,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(draft_collection_id)
    .bind(base_collection_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn clear_draft_tx(conn: &mut SqliteConnection, game_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE collection_runtime_state SET draft_collection_id = NULL, draft_base_collection_id = NULL, updated_at = CURRENT_TIMESTAMP WHERE game_id = ?",
    )
    .bind(game_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn clear_collection_references_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE collection_runtime_state SET
             active_collection_id = CASE WHEN active_collection_id = ? THEN NULL ELSE active_collection_id END,
             draft_collection_id = CASE WHEN draft_collection_id = ? THEN NULL ELSE draft_collection_id END,
             draft_base_collection_id = CASE WHEN draft_base_collection_id = ? THEN NULL ELSE draft_base_collection_id END,
             updated_at = CURRENT_TIMESTAMP
           WHERE active_collection_id = ? OR draft_collection_id = ? OR draft_base_collection_id = ?"#,
    )
    .bind(collection_id)
    .bind(collection_id)
    .bind(collection_id)
    .bind(collection_id)
    .bind(collection_id)
    .bind(collection_id)
    .execute(conn)
    .await?;
    Ok(())
}
