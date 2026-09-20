use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::modules::collections::application::collection::filter_target_mods_for_safe_mode;
use crate::shared::errors::CollectionError;
use crate::shared::path_key::folder_path_key;

/// Load the target collection's members.
pub async fn resolve(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let collection = ctx.collection()?.clone();
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let snapshot =
        crate::modules::collections::application::collection::load_projected_collection_state(
            &ctx.pool,
            &collection,
            Some(mods_path.as_str()),
        )
        .await?;
    let (mods, objects) =
        crate::modules::collections::application::collection::collection_members_from_projected_state(
            &ctx.collection_id,
            &snapshot,
        );
    let requested_mod_count = mods.len();
    ctx.safe_mode_scope_path_keys = mods
        .iter()
        .map(|member| {
            member
                .mod_path_key
                .clone()
                .unwrap_or_else(|| folder_path_key(&member.mod_path, None))
        })
        .collect();
    ctx.target_mods = filter_target_mods_for_safe_mode(mods, ctx.safe_mode);
    ctx.target_objects = objects;

    let excluded_unsafe = requested_mod_count.saturating_sub(ctx.target_mods.len());
    if excluded_unsafe > 0 {
        ctx.warnings.push(format!(
            "Safe Mode kept {excluded_unsafe} unsafe or unclassified managed mod(s) disabled"
        ));
    }

    log::info!(
        "apply_pipeline[resolve_target]: loaded {} mods and {} objects for collection '{}'",
        ctx.target_mods.len(),
        ctx.target_objects.len(),
        ctx.collection_id
    );

    Ok(())
}
