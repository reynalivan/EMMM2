use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::corridor_repo;

/// Step 8: Update corridor pointers to reflect the applied collection.
pub async fn update(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    // Compute new signature from target mods
    let mut path_keys: Vec<&str> = ctx
        .target_mods
        .iter()
        .map(|m| m.mod_path.as_str())
        .collect();
    path_keys.sort();
    let signature = blake3::hash(path_keys.join("\n").as_bytes())
        .to_hex()
        .to_string();
    ctx.new_signature = signature.clone();

    corridor_repo::update_pointers(
        &ctx.pool,
        &ctx.game_id,
        ctx.is_safe,
        Some(&ctx.collection_id),
        ctx.undo_snapshot_id.as_deref(),
    )
    .await?;

    // Also update runtime cache signature
    sqlx::query(
        "UPDATE corridor_runtime_cache SET signature = ? WHERE game_id = ? AND is_safe = ?",
    )
    .bind(&signature)
    .bind(&ctx.game_id)
    .bind(if ctx.is_safe { 1i32 } else { 0i32 })
    .execute(&ctx.pool)
    .await?;

    log::info!(
        "apply_pipeline[update_corridor]: corridor pointers updated (active='{}', undo={:?}, sig='{}')",
        ctx.collection_id,
        ctx.undo_snapshot_id,
        &signature[..8.min(signature.len())]
    );

    Ok(())
}
