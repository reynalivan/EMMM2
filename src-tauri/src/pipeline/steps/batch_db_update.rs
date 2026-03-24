use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;

/// Step 7: Batch update the database to reflect FS renames.
///
/// Updates `mods.status` and `mods.disabled_reason` for all affected mods
/// in two batch queries (one for enables, one for disables).
pub async fn update(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    // Batch enable in DB
    if !ctx.to_enable.is_empty() {
        let keys_json = serde_json::to_string(&ctx.to_enable).unwrap_or_default();
        sqlx::query(
            r#"UPDATE mods SET status = 1, disabled_reason = NULL, updated_at = CURRENT_TIMESTAMP
            WHERE game_id = ? AND folder_path_key IN (SELECT value FROM json_each(?))"#,
        )
        .bind(&ctx.game_id)
        .bind(&keys_json)
        .execute(&ctx.pool)
        .await?;
    }

    // Batch disable in DB
    if !ctx.to_disable.is_empty() {
        let keys_json = serde_json::to_string(&ctx.to_disable).unwrap_or_default();
        sqlx::query(
            r#"UPDATE mods SET status = 0, disabled_reason = 'COLLECTION', updated_at = CURRENT_TIMESTAMP
            WHERE game_id = ? AND folder_path_key IN (SELECT value FROM json_each(?))"#,
        )
        .bind(&ctx.game_id)
        .bind(&keys_json)
        .execute(&ctx.pool)
        .await?;
    }

    log::info!(
        "apply_pipeline[batch_db_update]: {} enabled, {} disabled in DB",
        ctx.to_enable.len(),
        ctx.to_disable.len()
    );

    Ok(())
}
