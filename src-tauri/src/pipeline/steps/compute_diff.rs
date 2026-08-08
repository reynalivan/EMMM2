use crate::common::path_key::folder_path_key;
use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use std::collections::HashSet;

/// Compute the diff between current state and target collection.
pub async fn compute(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    // Both sides of the diff must be in the canonical key space —
    // `currently_enabled_path_keys` holds canonical root keys, so a raw display
    // path here could only ever land in `to_enable`.
    let target_keys: HashSet<String> = ctx
        .target_mods
        .iter()
        .map(|member| {
            member
                .mod_path_key
                .clone()
                .unwrap_or_else(|| folder_path_key(&member.mod_path, None))
        })
        .collect();

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
