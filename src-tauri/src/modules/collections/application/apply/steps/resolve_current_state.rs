use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::shared::errors::CollectionError;

/// Resolve currently-enabled mod path keys within this apply operation's scope.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let (mods, objects) =
        crate::modules::collections::application::collection::load_live_active_projection_state(
            &ctx.pool,
            &ctx.game_id,
        )
        .await?;
    ctx.currently_enabled_path_keys =
        crate::modules::workspace::application::projected_state::active_root_keys(
            &mods, &objects, None,
        )
        .into_iter()
        .filter(|path_key| {
            !ctx.restrict_current_state_to_target_scope
                || ctx.safe_mode_scope_path_keys.contains(path_key)
        })
        .collect();

    log::info!(
        "apply_pipeline[resolve_current]: {} mods currently enabled in runtime",
        ctx.currently_enabled_path_keys.len()
    );

    Ok(())
}
