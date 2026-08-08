use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;

/// Record apply result metadata without mutating legacy corridor pointers.
pub async fn update(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let signature = crate::services::collection_service::compute_signature(
        &ctx.target_mods,
        &ctx.target_objects,
    );
    ctx.new_signature = signature.clone();
    let collection_name = ctx.collection()?.name.clone();
    ctx.final_state_is_dirty = !ctx.skipped_missing_paths.is_empty();
    ctx.final_state_name = Some(collection_name);

    log::info!(
        "apply_pipeline[update_corridor]: collection '{}' applied without legacy pointer mutation (sig='{}')",
        ctx.collection_id,
        &signature[..8.min(signature.len())]
    );

    Ok(())
}
