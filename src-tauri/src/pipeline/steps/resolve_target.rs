use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::collection_repo;

/// Step 2: Load the target collection's members.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let collection = collection_repo::get_by_id(&ctx.pool, &ctx.collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: ctx.collection_id.clone(),
        })?;
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let snapshot = crate::services::collection_service::load_projected_collection_state(
        &ctx.pool,
        &collection,
        Some(mods_path.as_str()),
    )
    .await?;
    let (mods, objects, _) =
        crate::services::collection_service::collection_members_from_projected_state(
            &ctx.collection_id,
            collection.is_safe,
            &snapshot,
        );
    ctx.target_mods = mods;
    ctx.target_objects = objects;

    log::info!(
        "apply_pipeline[resolve_target]: loaded {} mods and {} objects for collection '{}'",
        ctx.target_mods.len(),
        ctx.target_objects.len(),
        ctx.collection_id
    );

    Ok(())
}
