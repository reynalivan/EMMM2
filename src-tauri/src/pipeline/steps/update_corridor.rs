use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::collection_repo;

/// Step 8: Record apply result metadata without mutating legacy corridor pointers.
pub async fn update(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let signature = crate::services::collection_service::compute_signature(
        &ctx.target_mods,
        &ctx.target_objects,
    );
    ctx.new_signature = signature.clone();
    let collection = collection_repo::get_by_id(&ctx.pool, &ctx.collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: ctx.collection_id.clone(),
        })?;
    ctx.final_state_is_dirty = !ctx.skipped_missing_paths.is_empty();
    ctx.final_state_name = Some(collection.name.clone());

    log::info!(
        "apply_pipeline[update_corridor]: collection '{}' applied without legacy pointer mutation (sig='{}')",
        ctx.collection_id,
        &signature[..8.min(signature.len())]
    );

    Ok(())
}
