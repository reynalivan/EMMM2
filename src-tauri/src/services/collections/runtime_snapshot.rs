use super::root_resolution::{
    display_name_for_path, find_root_preview_path, resolve_existing_preview_path,
};
use super::types::{
    CanonicalCollectionSnapshot, CollectionPreviewMod, CollectionRuntimePreview,
    CollectionRuntimeSummary, CollectionStateKind, CorridorRuntimeSnapshot, RuntimeObjectState,
};
use crate::database::{
    collection_repo, corridor_runtime_cache_repo, corridor_state_repo, game_repo, object_repo,
};
use crate::services::corridor_constants::CORRIDOR_UNSAVED_PRESET_LABEL;
use crate::services::path_key::{
    canonical_collection_path_key, folder_path_key, object_name_key, resolve_collection_path,
};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Clone)]
struct ObjectLookup {
    by_folder_key: HashMap<String, object_repo::ObjectRuntimeDescriptor>,
    by_name_key: HashMap<String, object_repo::ObjectRuntimeDescriptor>,
}

pub(crate) async fn get_corridor_runtime_snapshot(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CorridorRuntimeSnapshot, String> {
    let _ = materialize_game_collections_if_missing(pool, game_id).await?;
    let current_state =
        super::resolve_current_effective_corridor_state(pool, game_id, is_safe).await?;
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let object_states =
        build_runtime_object_states_for_game(pool, game_id, &current_state.object_states).await?;

    let effective_db_paths =
        collection_repo::get_mod_paths_for_ids_pool(pool, &current_state.effective_db_mod_ids)
            .await
            .map_err(|e| e.to_string())?;
    let mut candidate_paths: Vec<String> = effective_db_paths.into_values().collect();
    candidate_paths.extend(current_state.effective_nested_paths.iter().cloned());
    let roots = build_runtime_roots_for_game(pool, game_id, is_safe, candidate_paths).await?;
    let signature = serialize_signature(&roots, &object_states, mods_path.as_deref());
    let (active_collection_id, state_name, state_kind) =
        resolve_matched_collection(pool, game_id, is_safe, &signature).await?;

    let snapshot = CorridorRuntimeSnapshot {
        game_id: game_id.to_string(),
        is_safe,
        active_collection_id,
        state_name,
        state_kind,
        roots,
        object_states,
        signature,
        snapshot_source: "disk_scan".to_string(),
        reconciled_count: 0,
    };

    corridor_runtime_cache_repo::upsert_runtime_snapshot(pool, &snapshot)
        .await
        .map_err(|e| e.to_string())?;

    Ok(snapshot)
}

pub async fn get_collection_runtime_preview(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<CollectionRuntimePreview, String> {
    ensure_collection_runtime_materialized(pool, collection_id, game_id).await?;

    let details = collection_repo::get_collection_details(pool, collection_id, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Collection not found".to_string())?;
    let snapshot = collection_repo::get_collection_snapshot(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    let snapshot = snapshot.ok_or_else(|| "Collection snapshot is missing".to_string())?;
    let signature = collection_repo::get_collection_signature(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    Ok(CollectionRuntimePreview {
        collection: details.collection,
        roots: snapshot.roots,
        object_states: snapshot.object_states,
        signature,
    })
}

pub(crate) async fn persist_collection_runtime_materialization(
    pool: &SqlitePool,
    collection_id: &str,
    roots: &[CollectionPreviewMod],
    object_states: &[RuntimeObjectState],
    mods_path: Option<&str>,
) -> Result<String, String> {
    let signature = serialize_signature(roots, object_states, mods_path);
    let snapshot = build_canonical_collection_snapshot(roots, object_states);
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    collection_repo::upsert_collection_snapshot(&mut tx, collection_id, &snapshot, &signature)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(signature)
}

pub(crate) async fn ensure_collection_runtime_materialized(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<(), String> {
    let details = collection_repo::get_collection_details(pool, collection_id, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Collection not found".to_string())?;
    let existing_snapshot = collection_repo::get_collection_snapshot(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    let existing_signature = collection_repo::get_collection_signature(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    if existing_signature.is_some()
        && (existing_snapshot.is_some() || details.collection.member_count == 0)
    {
        return Ok(());
    }

    let legacy_db_mod_ids =
        collection_repo::get_mod_ids_for_collection_in_game(pool, collection_id, game_id)
            .await
            .map_err(|e| e.to_string())?;
    let mut roots =
        collection_repo::get_collection_preview_mods_by_ids(pool, game_id, &legacy_db_mod_ids)
            .await
            .map_err(|e| e.to_string())?;
    let orphaned_paths: Vec<String> =
        collection_repo::get_collection_items_with_missing_mods(pool, collection_id, game_id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter_map(|(_, path)| path)
            .collect();
    let nested_paths = collection_repo::get_nested_collection_items(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut additional_paths = orphaned_paths;
    additional_paths.extend(nested_paths);
    let additional_roots = build_runtime_roots_for_game(
        pool,
        game_id,
        details.collection.is_safe_context,
        additional_paths,
    )
    .await?;
    let current_object_states = collection_repo::get_current_object_states_for_game(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let saved_object_states = collection_repo::get_collection_object_states(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;
    let normalized_object_states =
        normalize_object_states(saved_object_states, &current_object_states);
    let runtime_object_states =
        build_runtime_object_states_for_game(pool, game_id, &normalized_object_states).await?;
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let mut seen_root_keys = HashSet::new();
    roots.retain(|root| {
        canonical_collection_path_key(&root.folder_path, mods_path.as_deref())
            .is_none_or(|path_key| seen_root_keys.insert(path_key))
    });
    for root in additional_roots {
        let Some(path_key) = canonical_collection_path_key(&root.folder_path, mods_path.as_deref())
        else {
            roots.push(root);
            continue;
        };
        if seen_root_keys.insert(path_key) {
            roots.push(root);
        }
    }

    let _ = persist_collection_runtime_materialization(
        pool,
        collection_id,
        &roots,
        &runtime_object_states,
        mods_path.as_deref(),
    )
    .await?;

    Ok(())
}

pub(crate) async fn materialize_game_collections_if_missing(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<usize, String> {
    let safe_collections = collection_repo::list_collections(pool, game_id, true)
        .await
        .map_err(|e| e.to_string())?;
    let unsafe_collections = collection_repo::list_collections(pool, game_id, false)
        .await
        .map_err(|e| e.to_string())?;

    let mut materialized = 0usize;
    for collection in safe_collections.into_iter().chain(unsafe_collections) {
        let existing_snapshot = collection_repo::get_collection_snapshot(pool, &collection.id)
            .await
            .map_err(|e| e.to_string())?;
        let existing_signature = collection_repo::get_collection_signature(pool, &collection.id)
            .await
            .map_err(|e| e.to_string())?;
        let is_complete = existing_signature.is_some()
            && (existing_snapshot.is_some() || collection.member_count == 0);
        if is_complete {
            continue;
        }

        ensure_collection_runtime_materialized(pool, &collection.id, game_id).await?;
        materialized += 1;
    }

    Ok(materialized)
}

pub(crate) async fn build_runtime_roots_for_game(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    candidate_paths: Vec<String>,
) -> Result<Vec<CollectionPreviewMod>, String> {
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let object_descriptors = object_repo::get_runtime_descriptors(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    let object_lookup = build_object_lookup(&object_descriptors);
    Ok(resolve_runtime_roots(
        candidate_paths,
        is_safe,
        mods_path.as_deref(),
        &object_lookup,
    ))
}

pub(crate) async fn build_runtime_object_states_for_game(
    pool: &SqlitePool,
    game_id: &str,
    object_states: &[super::types::CollectionObjectState],
) -> Result<Vec<RuntimeObjectState>, String> {
    let object_descriptors = object_repo::get_runtime_descriptors(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(build_runtime_object_states(
        object_states,
        &object_descriptors,
    ))
}

fn resolve_runtime_roots(
    candidate_paths: Vec<String>,
    is_safe: bool,
    mods_path: Option<&str>,
    object_lookup: &ObjectLookup,
) -> Vec<CollectionPreviewMod> {
    let mut seen = HashSet::new();
    let mut roots = Vec::new();

    for candidate_path in candidate_paths {
        let Some(root) = resolve_runtime_root(&candidate_path, is_safe, mods_path, object_lookup)
        else {
            continue;
        };
        let Some(path_key) = canonical_collection_path_key(&root.folder_path, mods_path) else {
            continue;
        };
        if seen.insert(path_key) {
            roots.push(root);
        }
    }

    roots.sort_by(|left, right| left.folder_path.cmp(&right.folder_path));
    roots
}

fn resolve_runtime_root(
    candidate_path: &str,
    is_safe: bool,
    mods_path: Option<&str>,
    object_lookup: &ObjectLookup,
) -> Option<CollectionPreviewMod> {
    let resolved_path = resolve_existing_preview_path(candidate_path, mods_path)
        .or_else(|| resolve_collection_path(candidate_path, mods_path))?;
    let mods_root = Path::new(mods_path?);
    let (root_path, node_type) = find_root_preview_path(&resolved_path, mods_root)?;
    let folder_path = root_path.to_string_lossy().to_string();
    let object_descriptor = resolve_object_descriptor(&root_path, mods_root, object_lookup);

    Some(CollectionPreviewMod {
        id: format!("runtime-root:{folder_path}"),
        actual_name: display_name_for_path(&folder_path),
        folder_path,
        is_safe,
        object_id: object_descriptor.as_ref().map(|item| item.id.clone()),
        object_name: object_descriptor.as_ref().map(|item| item.name.clone()),
        object_type: object_descriptor
            .as_ref()
            .map(|item| item.object_type.clone()),
        node_type: Some(node_type.as_str().to_string()),
    })
}

fn resolve_object_descriptor(
    root_path: &Path,
    mods_root: &Path,
    object_lookup: &ObjectLookup,
) -> Option<object_repo::ObjectRuntimeDescriptor> {
    let relative = root_path.strip_prefix(mods_root).ok()?;
    let first_segment = relative
        .components()
        .next()?
        .as_os_str()
        .to_string_lossy()
        .to_string();
    let folder_key = folder_path_key(&first_segment, None);
    if let Some(descriptor) = object_lookup.by_folder_key.get(&folder_key) {
        return Some(descriptor.clone());
    }

    let name_key = object_name_key(&display_name_for_path(&first_segment));
    object_lookup.by_name_key.get(&name_key).cloned()
}

fn build_object_lookup(descriptors: &[object_repo::ObjectRuntimeDescriptor]) -> ObjectLookup {
    let by_folder_key = descriptors
        .iter()
        .cloned()
        .map(|descriptor| (descriptor.folder_path_key.clone(), descriptor))
        .collect();
    let by_name_key = descriptors
        .iter()
        .cloned()
        .map(|descriptor| (object_name_key(&descriptor.name), descriptor))
        .collect();

    ObjectLookup {
        by_folder_key,
        by_name_key,
    }
}

fn build_runtime_object_states(
    object_states: &[super::types::CollectionObjectState],
    descriptors: &[object_repo::ObjectRuntimeDescriptor],
) -> Vec<RuntimeObjectState> {
    let descriptor_by_id: HashMap<&str, &object_repo::ObjectRuntimeDescriptor> = descriptors
        .iter()
        .map(|descriptor| (descriptor.id.as_str(), descriptor))
        .collect();

    let mut runtime_states: Vec<RuntimeObjectState> = object_states
        .iter()
        .map(|state| {
            let descriptor = descriptor_by_id.get(state.object_id.as_str());
            RuntimeObjectState {
                object_id: state.object_id.clone(),
                name: descriptor
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| state.object_id.clone()),
                object_type: descriptor
                    .map(|item| item.object_type.clone())
                    .unwrap_or_else(|| "Other".to_string()),
                is_enabled: state.is_enabled,
                thumbnail_hint: descriptor.and_then(|item| item.thumbnail_path.clone()),
            }
        })
        .collect();
    runtime_states.sort_by(|left, right| left.name.cmp(&right.name));
    runtime_states
}

fn normalize_object_states(
    saved_object_states: Vec<super::types::CollectionObjectState>,
    current_object_states: &[super::types::CollectionObjectState],
) -> Vec<super::types::CollectionObjectState> {
    if current_object_states.is_empty() {
        return saved_object_states;
    }

    let saved_lookup: HashMap<String, bool> = saved_object_states
        .into_iter()
        .map(|state| (state.object_id, state.is_enabled))
        .collect();
    let mut normalized: Vec<super::types::CollectionObjectState> = current_object_states
        .iter()
        .map(|state| super::types::CollectionObjectState {
            object_id: state.object_id.clone(),
            is_enabled: saved_lookup
                .get(&state.object_id)
                .copied()
                .unwrap_or(state.is_enabled),
        })
        .collect();
    normalized.sort_by(|left, right| left.object_id.cmp(&right.object_id));
    normalized
}

fn build_canonical_collection_snapshot(
    roots: &[CollectionPreviewMod],
    object_states: &[RuntimeObjectState],
) -> CanonicalCollectionSnapshot {
    CanonicalCollectionSnapshot {
        roots: roots.to_vec(),
        object_states: object_states.to_vec(),
        summary: CollectionRuntimeSummary {
            root_count: roots.len(),
            object_count: object_states.len(),
        },
    }
}

fn serialize_signature(
    roots: &[CollectionPreviewMod],
    object_states: &[RuntimeObjectState],
    mods_path: Option<&str>,
) -> String {
    let mut enabled_object_ids: Vec<&str> = object_states
        .iter()
        .filter(|state| state.is_enabled)
        .map(|state| state.object_id.as_str())
        .collect();
    enabled_object_ids.sort_unstable();

    let mut root_keys: Vec<String> = roots
        .iter()
        .filter_map(|root| canonical_collection_path_key(&root.folder_path, mods_path))
        .collect();
    root_keys.sort();
    root_keys.dedup();

    format!(
        "objects:{}\nroots:{}",
        enabled_object_ids.join("|"),
        root_keys.join("|")
    )
}

async fn resolve_matched_collection(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    signature: &str,
) -> Result<(Option<String>, Option<String>, CollectionStateKind), String> {
    let remembered = corridor_state_repo::get_corridor_state(pool, game_id, is_safe)
        .await
        .map_err(|e| e.to_string())?;
    let collections = collection_repo::list_collections(pool, game_id, is_safe)
        .await
        .map_err(|e| e.to_string())?;
    for collection in collections.iter().filter(|item| !item.is_last_unsaved) {
        ensure_collection_runtime_materialized(pool, &collection.id, game_id).await?;
    }

    let matches =
        collection_repo::find_named_collections_by_signature(pool, game_id, is_safe, signature)
            .await
            .map_err(|e| e.to_string())?;
    let matched = matches
        .iter()
        .find(|(id, _)| remembered.active_collection_id.as_deref() == Some(id.as_str()))
        .cloned()
        .or_else(|| matches.into_iter().next());

    if let Some((id, name)) = matched {
        return Ok((Some(id), Some(name), CollectionStateKind::Named));
    }

    if let Some(active_collection_id) = remembered.active_collection_id.as_deref() {
        ensure_collection_runtime_materialized(pool, active_collection_id, game_id).await?;
        let remembered_signature =
            collection_repo::get_collection_signature(pool, active_collection_id)
                .await
                .map_err(|e| e.to_string())?;
        if remembered_signature.as_deref() == Some(signature) {
            let remembered_name = collections
                .iter()
                .find(|collection| collection.id == active_collection_id)
                .map(|collection| collection.name.clone());
            return Ok((
                Some(active_collection_id.to_string()),
                remembered_name,
                CollectionStateKind::Named,
            ));
        }
    }

    Ok((
        None,
        Some(CORRIDOR_UNSAVED_PRESET_LABEL.to_string()),
        CollectionStateKind::Unsaved,
    ))
}
