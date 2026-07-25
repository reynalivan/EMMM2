//! Auto-heal of collection member references when mods or objects move,
//! get renamed, or go missing on disk.

use super::path_transition::{
    classify_collection_path_transition, logical_collection_path, unique_reference_candidates,
    CollectionPathTransitionKind,
};
use super::projection::build_projected_state_from_members;
use crate::domain::collection::{CollectionPathRewrite, CollectionReferenceImpact};
use crate::domain::errors::CollectionError;
use crate::repo::collection_repo;
use crate::services::projected_state_service;
use sqlx::SqlitePool;

pub async fn handle_mod_moved_or_renamed(
    pool: &SqlitePool,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<CollectionReferenceImpact, CollectionError> {
    let mut tx = pool.begin().await?;
    let count =
        handle_mod_moved_or_renamed_tx(&mut tx, old_mod_path, new_mod_path, new_object_id).await?;
    tx.commit().await?;
    Ok(count)
}

pub async fn handle_mod_moved_or_renamed_tx(
    conn: &mut sqlx::SqliteConnection,
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
        let references = load_collection_references_tx(&mut *conn, &old_candidate).await?;
        let count = collection_repo::update_member_paths(
            &mut *conn,
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
    mod_path: &str,
) -> Result<CollectionReferenceImpact, CollectionError> {
    let mut tx = pool.begin().await?;
    let impact = handle_mod_missing_tx(&mut tx, mod_path).await?;
    tx.commit().await?;
    Ok(impact)
}

pub async fn handle_mod_missing_tx(
    conn: &mut sqlx::SqliteConnection,
    mod_path: &str,
) -> Result<CollectionReferenceImpact, CollectionError> {
    let affected_collections = load_collection_references_tx(&mut *conn, mod_path).await?;
    if affected_collections.is_empty() {
        return Ok(CollectionReferenceImpact::default());
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
            let rows = collection_repo::find_mods_with_path_prefix(&mut *conn, &old_sep).await?;

            for (col_id, collection_name, old_path) in rows {
                let new_path = old_path.replacen(&old_sep, &new_sep, 1);

                collection_repo::rewrite_member_path(
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
    mod_path: &str,
) -> Result<Vec<CollectionReferenceRow>, CollectionError> {
    let references = collection_repo::get_references_by_mod_path(&mut *conn, mod_path)
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
    let (is_safe, mods_path) =
        collection_repo::get_corridor_context(&mut *conn, collection_id).await?;
    let mods = collection_repo::get_mods_tx(&mut *conn, collection_id).await?;
    let objects = collection_repo::get_objects_tx(&mut *conn, collection_id).await?;
    let projected_state = build_projected_state_from_members(&mods, &objects, mods_path.as_deref());
    let signature = projected_state_service::signature_for_projected_state(&projected_state);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state);
    let roots = projected_state_service::roots_from_projected_state(
        collection_id,
        is_safe != 0,
        &projected_state,
    );
    collection_repo::replace_all_state_tx(
        &mut *conn,
        collection_id,
        &mods,
        &objects,
        &roots,
        Some(&signature),
        snapshot_json.as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await?;

    Ok(())
}
