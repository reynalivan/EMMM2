use crate::shared::errors::AppError;

pub(crate) async fn read_projection_checkpoint(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
) -> Result<u64, AppError> {
    let revision = sqlx::query_scalar::<_, i64>(
        "SELECT projected_revision FROM workspace_projection_checkpoints WHERE game_id = ? AND source_epoch = ?",
    )
    .bind(game_id)
    .bind(source_epoch)
    .fetch_optional(pool)
    .await?;
    Ok(revision
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0))
}

pub(crate) async fn persist_projection_checkpoint(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    source_epoch: &str,
    revision: u64,
) -> Result<(), AppError> {
    let revision = i64::try_from(revision)
        .map_err(|_| AppError::Validation("Disk revision exceeds SQLite range".to_string()))?;
    sqlx::query(
        "INSERT INTO workspace_projection_checkpoints (game_id, source_epoch, projected_revision) VALUES (?, ?, ?) \
         ON CONFLICT(game_id, source_epoch) DO UPDATE SET projected_revision = MAX(projected_revision, excluded.projected_revision)",
    )
    .bind(game_id)
    .bind(source_epoch)
    .bind(revision)
    .execute(pool)
    .await?;
    Ok(())
}
