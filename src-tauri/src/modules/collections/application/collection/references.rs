//! Auto-heal of collection member references when mods or objects move,
//! get renamed, or go missing on disk.

use super::path_transition::{
    classify_collection_path_transition, logical_collection_path, unique_reference_candidates,
    CollectionPathTransitionKind,
};
use super::projection::{persist_projected_state, refresh_projected_state_preserving_signature};
use crate::modules::collections::domain::collection::{CollectionPathRewrite, CollectionReferenceImpact};
use crate::shared::errors::CollectionError;
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::workspace::application::projected_state;
use sqlx::SqlitePool;

pub(crate) struct StagedCollectionIdentityTransitions {
    pub(crate) impact: CollectionReferenceImpact,
    pub(crate) affected_collection_ids: Vec<String>,
}

fn path_with_reference_separator(path: &str, reference: &str) -> String {
    if reference.contains('/') {
        path.replace('\\', "/")
    } else if reference.contains('\\') {
        path.replace('/', "\\")
    } else {
        path.to_string()
    }
}

fn rewrite_descendant_path(path: &str, old_root: &str, new_root: &str) -> Option<String> {
    let path_key = crate::shared::path_key::folder_path_key(path, None);
    let old_key = crate::shared::path_key::folder_path_key(old_root, None);
    if path_key == old_key {
        return Some(path_with_reference_separator(new_root, path));
    }
    let key_prefix = format!("{old_key}/");
    if !path_key.starts_with(&key_prefix) {
        return None;
    }

    let root_component_count = old_root
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .count();
    let mut seen_components = 1;
    for (index, separator) in path.char_indices() {
        if !matches!(separator, '/' | '\\') {
            continue;
        }
        if seen_components == root_component_count {
            let remainder = &path[index + separator.len_utf8()..];
            let new_root = path_with_reference_separator(new_root, path);
            return Some(format!("{new_root}{separator}{remainder}"));
        }
        seen_components += 1;
    }
    None
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
        for (id, name) in collection::stage_object_reference(
            &mut *conn,
            game_id,
            &old_ref_key,
            &staged_ref_key,
        )
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
            Some(path_with_reference_separator(&rewrite.to, &member.mod_path))
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

pub(crate) async fn refresh_collection_signatures_tx(
    conn: &mut sqlx::SqliteConnection,
    collection_ids: &[String],
) -> Result<(), CollectionError> {
    for collection_id in collection_ids {
        recompute_signature_tx(&mut *conn, collection_id).await?;
    }
    Ok(())
}

pub async fn handle_mod_moved_or_renamed(
    pool: &SqlitePool,
    game_id: &str,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<CollectionReferenceImpact, CollectionError> {
    let mut tx = pool.begin().await?;
    let count =
        handle_mod_moved_or_renamed_tx(&mut tx, game_id, old_mod_path, new_mod_path, new_object_id)
            .await?;
    tx.commit().await?;
    Ok(count)
}

pub async fn handle_mod_moved_or_renamed_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<CollectionReferenceImpact, CollectionError> {
    if classify_collection_path_transition(old_mod_path, new_mod_path)
        == CollectionPathTransitionKind::RuntimeTogglePrefix
    {
        return Ok(CollectionReferenceImpact::default());
    }

    let new_logical_path = logical_collection_path(new_mod_path);
    let mut affected_collections =
        std::collections::BTreeMap::<String, CollectionReferenceRow>::new();
    let mut rewritten_paths = Vec::new();

    for old_candidate in unique_reference_candidates(old_mod_path) {
        let references = load_collection_references_tx(&mut *conn, game_id, &old_candidate).await?;
        let count = collection::update_member_paths(
            &mut *conn,
            game_id,
            &old_candidate,
            &new_logical_path,
            new_object_id,
        )
        .await?;

        if count == 0 {
            continue;
        }

        for collection in references {
            affected_collections.insert(collection.id.clone(), collection);
        }
        rewritten_paths.push(CollectionPathRewrite {
            from: old_candidate,
            to: new_logical_path.clone(),
        });
    }

    if affected_collections.is_empty() {
        return Ok(CollectionReferenceImpact::default());
    }

    for collection in affected_collections.values() {
        recompute_signature_tx(&mut *conn, &collection.id).await?;
    }

    Ok(CollectionReferenceImpact {
        affected_collection_count: affected_collections.len(),
        affected_collection_names: affected_collections
            .values()
            .map(|entry| entry.name.clone())
            .collect(),
        rewritten_paths,
        missing_paths: Vec::new(),
    })
}

pub async fn handle_mod_missing(
    pool: &SqlitePool,
    game_id: &str,
    mod_path: &str,
) -> Result<CollectionReferenceImpact, CollectionError> {
    let mut tx = pool.begin().await?;
    let impact = handle_mod_missing_tx(&mut tx, game_id, mod_path).await?;
    tx.commit().await?;
    Ok(impact)
}

pub async fn handle_mod_missing_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mod_path: &str,
) -> Result<CollectionReferenceImpact, CollectionError> {
    let affected_collections = load_collection_references_tx(&mut *conn, game_id, mod_path).await?;
    if affected_collections.is_empty() {
        return Ok(CollectionReferenceImpact::default());
    }

    for collection in &affected_collections {
        refresh_missing_projection_tx(&mut *conn, &collection.id).await?;
    }

    Ok(CollectionReferenceImpact {
        affected_collection_count: affected_collections.len(),
        affected_collection_names: affected_collections
            .iter()
            .map(|entry| entry.name.clone())
            .collect(),
        rewritten_paths: Vec::new(),
        missing_paths: vec![mod_path.to_string()],
    })
}

pub async fn handle_object_renamed_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    old_object_folder: &str,
    new_object_folder: &str,
) -> Result<CollectionReferenceImpact, CollectionError> {
    if classify_collection_path_transition(old_object_folder, new_object_folder)
        == CollectionPathTransitionKind::RuntimeTogglePrefix
    {
        return Ok(CollectionReferenceImpact::default());
    }

    let new_logical_folder = logical_collection_path(new_object_folder);
    let mut affected_collections =
        std::collections::BTreeMap::<String, CollectionReferenceRow>::new();

    for old_candidate in unique_reference_candidates(old_object_folder) {
        let old_ref_key = crate::shared::path_key::folder_path_key(&old_candidate, None);
        let new_ref_key = crate::shared::path_key::folder_path_key(&new_logical_folder, None);
        for (id, name) in collection::rewrite_object_references(
            &mut *conn,
            game_id,
            &old_ref_key,
            &new_ref_key,
            &crate::modules::workspace::domain::normalizer::normalize_display_name(&new_logical_folder),
        )
        .await?
        {
            affected_collections.insert(id.clone(), CollectionReferenceRow { id, name });
        }
        for (old_sep, new_sep) in [
            (
                format!("{}\\", old_candidate),
                format!("{}\\", new_logical_folder),
            ),
            (
                format!("{}/", old_candidate),
                format!("{}/", new_logical_folder),
            ),
        ] {
            let rows =
                collection::find_mods_with_path_prefix(&mut *conn, game_id, &old_sep).await?;

            for (col_id, collection_name, old_path) in rows {
                let new_path = old_path.replacen(&old_sep, &new_sep, 1);

                collection::rewrite_member_path(
                    &mut *conn, &col_id, &old_path, &new_path, &old_sep, &new_sep,
                )
                .await?;

                affected_collections.insert(
                    col_id.clone(),
                    CollectionReferenceRow {
                        id: col_id,
                        name: collection_name,
                    },
                );
            }
        }
    }

    if affected_collections.is_empty() {
        return Ok(CollectionReferenceImpact::default());
    }

    for collection in affected_collections.values() {
        recompute_signature_tx(&mut *conn, &collection.id).await?;
    }

    Ok(CollectionReferenceImpact {
        affected_collection_count: affected_collections.len(),
        affected_collection_names: affected_collections
            .values()
            .map(|entry| entry.name.clone())
            .collect(),
        rewritten_paths: vec![CollectionPathRewrite {
            from: logical_collection_path(old_object_folder),
            to: new_logical_folder,
        }],
        missing_paths: Vec::new(),
    })
}

#[derive(Debug, Clone)]
struct CollectionReferenceRow {
    id: String,
    name: String,
}

async fn load_collection_references_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mod_path: &str,
) -> Result<Vec<CollectionReferenceRow>, CollectionError> {
    let references = collection::get_references_by_mod_path(&mut *conn, game_id, mod_path)
        .await?
        .into_iter()
        .map(|(id, name)| CollectionReferenceRow { id, name })
        .collect();

    Ok(references)
}

async fn recompute_signature_tx(
    conn: &mut sqlx::SqliteConnection,
    collection_id: &str,
) -> Result<(), CollectionError> {
    let (_, mods_path) = collection::get_projection_context(&mut *conn, collection_id).await?;
    let mods = collection::get_mods_tx(&mut *conn, collection_id).await?;
    let objects = collection::get_objects_tx(&mut *conn, collection_id).await?;
    let projected_state =
        projected_state::build_projected_state(&mods, &objects, mods_path.as_deref());
    persist_projected_state(&mut *conn, collection_id, &mods, &objects, &projected_state).await
}

async fn refresh_missing_projection_tx(
    conn: &mut sqlx::SqliteConnection,
    collection_id: &str,
) -> Result<(), CollectionError> {
    let (_, mods_path) = collection::get_projection_context(&mut *conn, collection_id).await?;
    let mods = collection::get_mods_tx(&mut *conn, collection_id).await?;
    let objects = collection::get_objects_tx(&mut *conn, collection_id).await?;
    let projected_state =
        projected_state::build_projected_state(&mods, &objects, mods_path.as_deref());
    refresh_projected_state_preserving_signature(
        &mut *conn,
        collection_id,
        &mods,
        &objects,
        &projected_state,
    )
    .await
}
