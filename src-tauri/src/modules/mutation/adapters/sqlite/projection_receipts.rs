use sqlx::{SqliteConnection, SqlitePool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionReceipt {
    pub operation_id: String,
    pub game_id: String,
    pub source_epoch: String,
}

pub async fn insert_tx(
    conn: &mut SqliteConnection,
    receipt: &ProjectionReceipt,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO mutation_projection_receipts (operation_id, game_id, source_epoch) VALUES (?, ?, ?)",
    )
    .bind(&receipt.operation_id)
    .bind(&receipt.game_id)
    .bind(&receipt.source_epoch)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get(
    pool: &SqlitePool,
    operation_id: &str,
) -> Result<Option<ProjectionReceipt>, sqlx::Error> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT operation_id, game_id, source_epoch FROM mutation_projection_receipts WHERE operation_id = ?",
    )
    .bind(operation_id)
    .fetch_optional(pool)
    .await?;
    Ok(
        row.map(|(operation_id, game_id, source_epoch)| ProjectionReceipt {
            operation_id,
            game_id,
            source_epoch,
        }),
    )
}

pub async fn delete(pool: &SqlitePool, operation_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mutation_projection_receipts WHERE operation_id = ?")
        .bind(operation_id)
        .execute(pool)
        .await?;
    Ok(())
}
