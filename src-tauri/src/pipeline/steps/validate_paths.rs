use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::DISABLED_PREFIX;
use std::path::Path;

/// Pre-Apply Validation Step: Verify physical paths exist for all target mod members.
pub async fn validate(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let mods_path = &ctx.mods_path;

    let mut missing_paths: Vec<String> = Vec::new();
    let mut valid_mods = Vec::new();

    // Only validate mods — objects are top-level Containers and always exist physically
    // (if they don't, the scanner would have GC'd them or they'll be empty, which is fine)

    for m in &ctx.target_mods {
        let path_key = &m.mod_path;
        let candidate = mods_path.join(path_key);

        if candidate.exists() {
            valid_mods.push(m.clone());
            continue;
        }

        // Try with DISABLED prefix
        let file_name = Path::new(path_key)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let parent = Path::new(path_key).parent().unwrap_or(Path::new(""));
        let disabled_name = format!("{}{}", DISABLED_PREFIX, file_name);
        let disabled_candidate = mods_path.join(parent).join(&disabled_name);

        if disabled_candidate.exists() {
            valid_mods.push(m.clone());
        } else {
            missing_paths.push(path_key.clone());
        }
    }

    if !missing_paths.is_empty() && !ctx.ignore_missing {
        return Err(CollectionError::MissingMods {
            count: missing_paths.len(),
            paths: missing_paths,
        });
    }

    if !missing_paths.is_empty() {
        log::warn!(
            "apply_pipeline[validate_paths]: {} mod(s) missing from disk: {:?}",
            missing_paths.len(),
            missing_paths
        );
        ctx.skipped_missing_paths = missing_paths.clone();
        ctx.warnings.extend(
            missing_paths
                .iter()
                .map(|path| format!("Missing mod on disk: {path}")),
        );
        ctx.target_mods = valid_mods;
    }

    log::info!(
        "apply_pipeline[validate_paths]: proceeding with {} target mods",
        ctx.target_mods.len()
    );

    Ok(())
}
