use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::shared::errors::CollectionError;

/// Resolve currently-enabled mod path keys for the whole runtime.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let (mods, objects) =
        crate::modules::collections::application::collection::load_live_runtime_state(
            &ctx.pool,
            &ctx.game_id,
        )
        .await?;
    let projected_state =
        crate::modules::workspace::application::projected_state::build_projected_state(
            &mods, &objects, None,
        );
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
