use crate::modules::collections::domain::collection::CollectionMod;
use crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN;

/// Applies the same Safe Mode eligibility rule used by collection apply and preview.
pub(crate) fn filter_target_mods_for_safe_mode(
    mods: Vec<CollectionMod>,
    safe_mode: bool,
) -> Vec<CollectionMod> {
    if !safe_mode {
        return mods;
    }

    mods.into_iter()
        .filter(|member| is_safe_mode_eligible(member.is_safe, member.safety_source.as_deref()))
        .collect()
}

pub(crate) fn is_safe_mode_eligible(is_safe: bool, safety_source: Option<&str>) -> bool {
    is_safe && safety_source.is_some_and(|source| source != SAFETY_SOURCE_UNKNOWN)
}

pub(crate) async fn constrain_runtime_snapshot_safety(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    state: &mut crate::modules::collections::domain::collection::ProjectedCollectionState,
) -> Result<(), crate::shared::errors::CollectionError> {
    let keys = state
        .active_roots
        .iter()
        .map(|root| root.root_key.clone())
        .collect::<Vec<_>>();
    let mut conn = pool.acquire().await?;
    let rows = crate::modules::library::adapters::sqlite::mods::get_rows_for_reconcile_scope(
        &mut conn,
        game_id,
        &keys,
        &[],
    )
    .await?;
    let by_key = rows
        .into_iter()
        .map(|row| (row.folder_path_key, (row.is_safe, row.safety_source)))
        .collect::<std::collections::HashMap<_, _>>();
    for root in &mut state.active_roots {
        if let Some((is_safe, source)) = by_key.get(&root.root_key) {
            root.is_safe &= *is_safe;
            if !is_safe_mode_eligible(*is_safe, source.as_deref()) {
                root.safety_source.clone_from(source);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::collections::domain::collection::MemberKind;

    fn member(name: &str, is_safe: bool, safety_source: Option<&str>) -> CollectionMod {
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
        let target = filter_target_mods_for_safe_mode(
            vec![
                member("safe", true, Some("manual")),
                member("unsafe", false, Some("manual")),
                member("unknown", true, Some(SAFETY_SOURCE_UNKNOWN)),
                member("unclassified", true, None),
            ],
            true,
        );

        assert_eq!(target.len(), 1);
        assert_eq!(target[0].display_name.as_deref(), Some("safe"));
    }

    #[test]
    fn normal_mode_keeps_the_requested_selection() {
        let target = filter_target_mods_for_safe_mode(
            vec![
                member("safe", true, Some("manual")),
                member("unsafe", false, Some("manual")),
            ],
            false,
        );

        assert_eq!(target.len(), 2);
    }
}
