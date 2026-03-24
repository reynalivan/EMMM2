use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::collection_repo;

/// Step 2: Load the target collection's members.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    ctx.target_mods = collection_repo::get_mods(&ctx.pool, &ctx.collection_id).await?;
    ctx.target_objects = collection_repo::get_objects(&ctx.pool, &ctx.collection_id).await?;

    log::info!(
        "apply_pipeline[resolve_target]: loaded {} mods and {} objects for collection '{}'",
        ctx.target_mods.len(),
        ctx.target_objects.len(),
        ctx.collection_id
    );

    Ok(())
}
