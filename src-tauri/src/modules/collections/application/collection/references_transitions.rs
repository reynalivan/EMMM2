use super::path_transition::rewrite_descendant_path;
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::collections::domain::collection::{
    CollectionPathRewrite, CollectionReferenceImpact,
};
use crate::shared::errors::CollectionError;
pub(crate) struct StagedCollectionIdentityTransitions {
    pub(crate) impact: CollectionReferenceImpact,
    pub(crate) affected_collection_ids: Vec<String>,
}

/// Rewrites every collection reference involved in filesystem-identity moves
/// as one two-phase batch. This is required for A <-> B swaps: no source may
/// claim its final key until every other source has left that namespace.
pub(crate) async fn stage_identity_path_transitions_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_rewrites: &[CollectionPathRewrite],
    mod_rewrites: &[CollectionPathRewrite],
) -> Result<StagedCollectionIdentityTransitions, CollectionError> {
    let all_rewrites = object_rewrites
        .iter()
        .chain(mod_rewrites)
        .cloned()
        .collect::<Vec<_>>();
    super::safe_mode_references::rewrite_safe_mode_references_tx(conn, game_id, &all_rewrites)
        .await?;
    let mut affected = std::collections::BTreeMap::<String, String>::new();

    let mut staged_objects = Vec::new();
    for rewrite in object_rewrites {
        let old_ref_key = crate::shared::path_key::folder_path_key(&rewrite.from, None);
        let final_ref_key = crate::shared::path_key::folder_path_key(&rewrite.to, None);
        if old_ref_key == final_ref_key {
            continue;
        }
        let staged_ref_key = format!(
            ".emmm-reconcile-collection-object-stage-{}",
            uuid::Uuid::new_v4()
        );
        for (id, name) in
            collection::stage_object_reference(&mut *conn, game_id, &old_ref_key, &staged_ref_key)
                .await?
        {
            affected.insert(id, name);
        }
        staged_objects.push((
            staged_ref_key,
            final_ref_key,
            crate::modules::workspace::domain::normalizer::normalize_display_name(&rewrite.to),
        ));
    }

    let exact_mod_rewrites = mod_rewrites
        .iter()
        .map(|rewrite| {
            (
                crate::shared::path_key::folder_path_key(&rewrite.from, None),
                rewrite,
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut object_rewrites_by_depth = object_rewrites.iter().collect::<Vec<_>>();
    object_rewrites_by_depth.sort_by_key(|rewrite| std::cmp::Reverse(rewrite.from.len()));

    let mut staged_members = Vec::new();
    for member in collection::list_member_paths_for_game(&mut *conn, game_id).await? {
        let member_key = crate::shared::path_key::folder_path_key(&member.mod_path, None);
        let final_path = if let Some(rewrite) = exact_mod_rewrites.get(&member_key) {
            Some(
                rewrite_descendant_path(&member.mod_path, &rewrite.from, &rewrite.to).ok_or_else(
                    || {
                        CollectionError::Validation(
                            "Could not rewrite collection member path".to_string(),
                        )
                    },
                )?,
            )
        } else {
            object_rewrites_by_depth.iter().find_map(|rewrite| {
                rewrite_descendant_path(&member.mod_path, &rewrite.from, &rewrite.to)
            })
        };
        let Some(final_path) = final_path else {
            continue;
        };
        if crate::shared::path_key::folder_path_key(&final_path, None) == member_key {
            continue;
        }
        let staged_path = format!(
            ".emmm-reconcile-collection-member-stage/{}",
            uuid::Uuid::new_v4()
        );
        collection::stage_member_path(
            &mut *conn,
            &member.collection_id,
            &member.mod_path,
            &staged_path,
        )
        .await?;
        affected.insert(member.collection_id.clone(), member.collection_name);
        staged_members.push((member.collection_id, staged_path, final_path));
    }

    for (staged_ref_key, final_ref_key, final_display_name) in staged_objects {
        collection::finalize_staged_object_reference(
            &mut *conn,
            game_id,
            &staged_ref_key,
            &final_ref_key,
            &final_display_name,
        )
        .await?;
    }
    for (collection_id, staged_path, final_path) in staged_members {
        collection::finalize_staged_member_path(
            &mut *conn,
            &collection_id,
            &staged_path,
            &final_path,
        )
        .await?;
    }

    let affected_collection_ids = affected.keys().cloned().collect::<Vec<_>>();
    let rewritten_paths = if affected.is_empty() {
        Vec::new()
    } else {
        object_rewrites
            .iter()
            .chain(mod_rewrites.iter())
            .cloned()
            .collect()
    };
    Ok(StagedCollectionIdentityTransitions {
        impact: CollectionReferenceImpact {
            affected_collection_count: affected.len(),
            affected_collection_names: affected.values().cloned().collect(),
            rewritten_paths,
            missing_paths: Vec::new(),
        },
        affected_collection_ids,
    })
}
