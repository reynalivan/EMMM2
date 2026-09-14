use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::shared::errors::CollectionError;
use crate::shared::path_key::folder_path_key;
use crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN;

fn filter_target_mods(
    mods: Vec<crate::modules::collections::domain::collection::CollectionMod>,
    safe_mode: bool,
) -> Vec<crate::modules::collections::domain::collection::CollectionMod> {
    if safe_mode {
        mods.into_iter()
            .filter(|member| {
                member.is_safe
                    && member
                        .safety_source
                        .as_deref()
                        .is_some_and(|source| source != SAFETY_SOURCE_UNKNOWN)
            })
            .collect()
    } else {
        mods
    }
}

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
    ctx.target_mods = filter_target_mods(mods, ctx.safe_mode);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::collections::domain::collection::{CollectionMod, MemberKind};

    fn mod_member(name: &str, is_safe: bool, safety_source: Option<&str>) -> CollectionMod {
        CollectionMod {
            kind: MemberKind::Mod,
            collection_id: "collection".to_string(),
            mod_id: Some(name.to_string()),
            mod_path: format!("library/{name}"),
            mod_path_key: None,
            object_id: "object".to_string(),
            display_name: Some(name.to_string()),
            preview_path: None,
            node_type: None,
            warnings: Vec::new(),
            is_enabled: true,
            is_safe,
            safety_source: safety_source.map(str::to_string),
        }
    }

    #[test]
    fn safe_mode_excludes_unsafe_and_unclassified_requested_mods() {
        let target = filter_target_mods(
            vec![
                mod_member("safe", true, Some("manual")),
                mod_member("unsafe", false, Some("manual")),
                mod_member("unknown", true, Some(SAFETY_SOURCE_UNKNOWN)),
                mod_member("unclassified", true, None),
            ],
            true,
        );

        assert_eq!(target.len(), 1);
        assert_eq!(target[0].display_name.as_deref(), Some("safe"));
    }

    #[test]
    fn safe_mode_off_keeps_the_requested_selection() {
        let target = filter_target_mods(
            vec![
                mod_member("safe", true, Some("manual")),
                mod_member("unsafe", false, Some("manual")),
            ],
            false,
        );

        assert_eq!(target.len(), 2);
    }
}
