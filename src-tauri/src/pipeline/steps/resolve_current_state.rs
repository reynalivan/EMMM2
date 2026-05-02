use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;

/// Step 3: Resolve the currently-enabled mod path_keys in this corridor.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let (mods, objects) =
        crate::services::collection_service::load_live_corridor_state(&ctx.pool, &ctx.game_id, ctx.is_safe)
            .await?;
    let projected_state =
        crate::services::projected_state_service::build_projected_state(&mods, &objects, None);
    ctx.currently_enabled_path_keys = projected_state
        .active_roots
        .into_iter()
        .map(|root| root.root_key)
        .collect();

    log::info!(
        "apply_pipeline[resolve_current]: {} mods currently enabled in corridor",
        ctx.currently_enabled_path_keys.len()
    );

    Ok(())
}
