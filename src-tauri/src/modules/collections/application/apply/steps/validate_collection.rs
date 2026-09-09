use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::modules::collections::application::collection::{
    require_collection, require_game_match,
};
use crate::shared::errors::CollectionError;

pub async fn validate(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let collection = require_collection(&ctx.pool, &ctx.collection_id).await?;
    require_game_match(&collection, &ctx.game_id)?;
    ctx.collection = Some(collection);
    Ok(())
}
