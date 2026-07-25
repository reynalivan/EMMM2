use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;

/// Step 3: Resolve currently-enabled mod path keys for the whole runtime.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let (mods, objects) =
        crate::services::collection_service::load_live_runtime_state(&ctx.pool, &ctx.game_id)
            .await?;
    let projected_state =
        crate::services::projected_state_service::build_projected_state(&mods, &objects, None);
    ctx.currently_enabled_path_keys = projected_state
        .active_roots
        .into_iter()
        .map(|root| root.root_key)
        .collect();

    log::info!(
        "apply_pipeline[resolve_current]: {} mods currently enabled in runtime",
        ctx.currently_enabled_path_keys.len()
    );

    Ok(())
}
