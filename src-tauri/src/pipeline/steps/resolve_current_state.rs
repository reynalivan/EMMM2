use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;

/// Step 3: Resolve the currently-enabled mod path_keys in this corridor.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let is_safe_i32 = if ctx.is_safe { 1i32 } else { 0i32 };

    let enabled_keys: Vec<String> = sqlx::query_scalar(
        r#"SELECT folder_path_key FROM mods
        WHERE game_id = ? AND is_safe = ? AND status = 1
        AND folder_path_key IS NOT NULL"#,
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .fetch_all(&ctx.pool)
    .await?;

    ctx.currently_enabled_path_keys = enabled_keys.into_iter().collect();

    log::info!(
        "apply_pipeline[resolve_current]: {} mods currently enabled in corridor",
        ctx.currently_enabled_path_keys.len()
    );

    Ok(())
}
