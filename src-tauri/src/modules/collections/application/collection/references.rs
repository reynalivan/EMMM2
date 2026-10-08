//! Auto-heal of collection member references when mods or objects move,
//! get renamed, or go missing on disk.

use super::path_transition::{
    classify_collection_path_transition, logical_collection_path, unique_reference_candidates,
    CollectionPathTransitionKind,
};
use super::projection::{persist_projected_state, refresh_projected_state_preserving_signature};
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::collections::domain::collection::{
    CollectionPathRewrite, CollectionReferenceImpact,
};
use crate::modules::workspace::application::projected_state;
use crate::shared::errors::CollectionError;
use sqlx::SqlitePool;

pub(crate) use super::references_transitions::stage_identity_path_transitions_tx;

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

    super::safe_mode_references::rewrite_safe_mode_references_tx(
        conn,
        game_id,
        &[CollectionPathRewrite {
            from: old_mod_path.to_string(),
            to: new_mod_path.to_string(),
        }],
    )
    .await?;
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

    super::safe_mode_references::rewrite_safe_mode_references_tx(
        conn,
        game_id,
        &[CollectionPathRewrite {
            from: old_object_folder.to_string(),
            to: new_object_folder.to_string(),
        }],
    )
    .await?;
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
            &crate::modules::workspace::domain::normalizer::normalize_display_name(
                &new_logical_folder,
            ),
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
