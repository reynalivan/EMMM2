use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::{corridor_label, ApplyContext};
use crate::services::collection_service::{require_collection, require_game_match};

/// Load the collection once and validate it belongs to the requested game.
pub async fn validate(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let collection = require_collection(&ctx.pool, &ctx.collection_id).await?;
    require_game_match(&collection, &ctx.game_id)?;

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
