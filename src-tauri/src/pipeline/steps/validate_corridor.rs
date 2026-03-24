use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::collection_repo;

/// Step 1: Validate that the collection belongs to the correct corridor.
pub async fn validate(ctx: &ApplyContext) -> Result<(), CollectionError> {
    let collection = collection_repo::get_by_id(&ctx.pool, &ctx.collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: ctx.collection_id.clone(),
        })?;

    if collection.is_safe != ctx.is_safe {
        return Err(CollectionError::Corridor(
            crate::domain::errors::CorridorError::CorridorMismatch {
                collection_mode: if collection.is_safe {
                    "Safe".to_string()
                } else {
                    "Unsafe".to_string()
                },
                current_mode: if ctx.is_safe {
                    "Safe".to_string()
                } else {
                    "Unsafe".to_string()
                },
            },
        ));
    }

    log::info!(
        "apply_pipeline[validate]: collection '{}' validated for {} corridor",
        collection.name,
        if ctx.is_safe { "Safe" } else { "Unsafe" }
    );

    Ok(())
}
