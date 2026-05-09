use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::corridor_repo;

/// Step 8: Update corridor pointers to reflect the applied collection.
pub async fn update(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let signature = crate::services::collection_service::compute_signature(
        &ctx.target_mods,
        &ctx.target_objects,
    );
    ctx.new_signature = signature.clone();
    let previous_state = corridor_repo::get(&ctx.pool, &ctx.game_id, ctx.is_safe).await?;
    let previous_active_collection_id = previous_state
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
        .filter(|collection_id| *collection_id != ctx.collection_id.as_str());

    corridor_repo::update_pointers(
        &ctx.pool,
        &ctx.game_id,
        ctx.is_safe,
        Some(&ctx.collection_id),
        previous_active_collection_id,
    )
    .await?;

    let snapshot =
        crate::services::corridor_service::get_corridor_state(&ctx.pool, &ctx.game_id, ctx.is_safe)
            .await
            .map_err(CollectionError::from)?;
    ctx.final_state_name = if snapshot.is_dirty || snapshot.active_collection_is_unsaved {
        Some(if ctx.is_safe {
            "Unsaved SAFE Preset".to_string()
        } else {
            "Unsaved UNSAFE Preset".to_string()
        })
    } else {
        snapshot.active_collection_name.clone()
    };

    log::info!(
        "apply_pipeline[update_corridor]: corridor pointers updated (active='{}', undo='{}', sig='{}')",
        ctx.collection_id,
        previous_active_collection_id.unwrap_or("NULL"),
        &signature[..8.min(signature.len())]
    );

    Ok(())
}
