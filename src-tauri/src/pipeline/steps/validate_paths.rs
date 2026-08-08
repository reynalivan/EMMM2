use std::collections::HashSet;

use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::services::mods::core_ops::resolve_existing_runtime_variant;

/// Pre-Apply Validation Step: Verify physical paths exist for all target mod members.
///
/// Resolution must match what `batch_rename` will actually do, so this uses the
/// same helper the mutation engine does. A weaker check here reports a mod as
/// missing that the rename step would have found — e.g. one under a
/// `disabled_`/`DISABLED-` spelling, a case-drifted segment, or a disabled
/// parent folder.
pub async fn validate(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let mods_path = &ctx.mods_path;

    // Only validate mods — objects are top-level Containers and always exist physically
    // (if they don't, the scanner would have GC'd them or they'll be empty, which is fine)
    let missing_paths: Vec<String> = ctx
        .target_mods
        .iter()
        .filter(|m| {
            resolve_existing_runtime_variant(mods_path, &mods_path.join(&m.mod_path), true)
                .is_none()
        })
        .map(|m| m.mod_path.clone())
        .collect();

    if missing_paths.is_empty() {
        log::info!(
            "apply_pipeline[validate_paths]: proceeding with {} target mods",
            ctx.target_mods.len()
        );
        return Ok(());
    }

    if !ctx.ignore_missing {
        return Err(CollectionError::MissingMods {
            count: missing_paths.len(),
            paths: missing_paths,
        });
    }

    log::warn!(
        "apply_pipeline[validate_paths]: {} mod(s) missing from disk: {:?}",
        missing_paths.len(),
        missing_paths
    );
    ctx.warnings.extend(
        missing_paths
            .iter()
            .map(|path| format!("Missing mod on disk: {path}")),
    );

    let missing: HashSet<&str> = missing_paths.iter().map(String::as_str).collect();
    ctx.target_mods
        .retain(|m| !missing.contains(m.mod_path.as_str()));
    ctx.skipped_missing_paths = missing_paths;

    log::info!(
        "apply_pipeline[validate_paths]: proceeding with {} target mods",
        ctx.target_mods.len()
    );

    Ok(())
}
