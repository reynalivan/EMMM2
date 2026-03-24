use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use std::collections::HashSet;

/// Step 4: Compute the diff between current state and target collection.
pub async fn compute(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    // Collect target path_keys from mods
    let target_keys: HashSet<String> = ctx.target_mods.iter().map(|m| m.mod_path.clone()).collect();

    // To enable: in target but not currently enabled
    ctx.to_enable = target_keys
        .difference(&ctx.currently_enabled_path_keys)
        .cloned()
        .collect();

    // To disable: currently enabled but not in target
    ctx.to_disable = ctx
        .currently_enabled_path_keys
        .difference(&target_keys)
        .cloned()
        .collect();

    log::info!(
        "apply_pipeline[compute_diff]: {} to enable, {} to disable",
        ctx.to_enable.len(),
        ctx.to_disable.len()
    );

    Ok(())
}
