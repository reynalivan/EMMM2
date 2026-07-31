use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::{corridor_label, ApplyContext};
use crate::repo::collection_repo;

/// Step 1: Load the collection once and validate it belongs to the requested game.
pub async fn validate(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let collection = collection_repo::get_by_id(&ctx.pool, &ctx.collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: ctx.collection_id.clone(),
        })?;

    if collection.game_id != ctx.game_id {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to game '{}'",
            ctx.collection_id, ctx.game_id
        )));
    }

    // Corridor enforcement: an unsafe collection is mathematically impossible to
    // apply while in Safe Mode, and vice versa. UI already scopes lists per
    // corridor; this is the backend guard so a direct invoke cannot bypass it.
    if collection.is_safe != ctx.is_safe {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' belongs to the {} corridor and cannot be applied in {} mode",
            ctx.collection_id,
            corridor_label(collection.is_safe),
            corridor_label(ctx.is_safe),
        )));
    }

    log::info!(
        "apply_pipeline[validate]: collection '{}' validated for game '{}'",
        collection.name,
        ctx.game_id
    );

    ctx.collection = Some(collection);

    Ok(())
}
