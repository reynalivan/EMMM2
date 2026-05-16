use crate::domain::collection::{
    ApplyPreview, ApplyResult, CollectionMod, CollectionObject, CollectionPathRewrite,
    CollectionPreview, CollectionReferenceImpact, CollectionRoot, CollectionSummary,
    CreateCollectionInput, CreateCollectionMode, ProjectedCollectionState, UpdateCollectionInput,
};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};
use crate::services::{
    collection_preview_tree::resolve_preview_terminal_metadata, projected_state_service,
};
use sqlx::{SqliteConnection, SqlitePool};

// ---------------------------------------------------------------------------
// collection_service — Business logic for collections
// ---------------------------------------------------------------------------

fn is_object_enabled(path_key: Option<&str>) -> bool {
    let Some(path_key) = path_key else {
        return true;
    };

    !path_key
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(is_disabled_folder)
}

fn build_projected_state_from_members(
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    mods_path: Option<&str>,
) -> ProjectedCollectionState {
    projected_state_service::build_projected_state(mods, objects, mods_path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollectionPathTransitionKind {
    RuntimeTogglePrefix,
    SemanticMoveOrRename,
}

fn logical_collection_path(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .map(normalize_display_name)
        .collect::<Vec<_>>()
        .join("/")
}

fn unique_reference_candidates(path: &str) -> Vec<String> {
    let logical_path = logical_collection_path(path);
    let mut candidates = vec![path.to_string()];
    if logical_path != path {
        candidates.push(logical_path);
    }
    candidates
}

pub(crate) fn classify_collection_path_transition(
    old_path: &str,
    new_path: &str,
) -> CollectionPathTransitionKind {
    if logical_collection_path(old_path) == logical_collection_path(new_path) {
        return CollectionPathTransitionKind::RuntimeTogglePrefix;
    }

    CollectionPathTransitionKind::SemanticMoveOrRename
}

pub(crate) async fn load_projected_collection_state(
    pool: &SqlitePool,
    collection: &crate::domain::collection::Collection,
    mods_path: Option<&str>,
) -> Result<ProjectedCollectionState, CollectionError> {
    if mods_path.is_none() {
        if let Some(snapshot_json) = collection.snapshot_json.as_deref() {
            if let Some(snapshot) = projected_state_service::parse_snapshot_json(snapshot_json) {
                let active_root_count = snapshot.summary.active_root_count as i32;
                if collection.root_count != active_root_count
                    || collection.display_mod_count != active_root_count
                {
                    sqlx::query(
                        "UPDATE collections SET root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    )
                    .bind(active_root_count)
                    .bind(active_root_count)
                    .bind(&collection.id)
                    .execute(pool)
                    .await?;
                }
                return Ok(snapshot);
            }
        }
    }

    let mods = collection_repo::get_mods(pool, &collection.id).await?;
    let objects = collection_repo::get_objects(pool, &collection.id).await?;
    let snapshot = build_projected_state_from_members(&mods, &objects, mods_path);
    if mods_path.is_some() {
        return Ok(snapshot);
    }

    let signature = projected_state_service::signature_for_projected_state(&snapshot);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&snapshot);

    sqlx::query(
        "UPDATE collections SET snapshot_json = ?, signature = ?, root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind(&signature)
    .bind(snapshot.summary.active_root_count as i32)
    .bind(snapshot.summary.active_root_count as i32)
    .bind(&collection.id)
    .execute(pool)
    .await?;

    Ok(snapshot)
}

pub(crate) fn collection_members_from_projected_state(
    collection_id: &str,
    is_safe: bool,
    state: &ProjectedCollectionState,
) -> (
    Vec<CollectionMod>,
    Vec<CollectionObject>,
    Vec<CollectionRoot>,
) {
    let mods = projected_state_service::mods_from_projected_state(collection_id, state);
    let objects = projected_state_service::objects_from_projected_state(collection_id, state);
    let roots = projected_state_service::roots_from_projected_state(collection_id, is_safe, state);
    (mods, objects, roots)
}

/// List all named collections for a game in the current corridor.
pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    _mods_path: Option<&str>,
) -> Result<Vec<CollectionSummary>, CollectionError> {
    let corridor_snapshot =
        crate::services::corridor_service::get_corridor_state(pool, game_id, is_safe)
            .await
            .map_err(CollectionError::Corridor)?;
    let active_id = corridor_snapshot.active_collection_id.as_deref();
    let undo_id = corridor_snapshot.undo_collection_id.as_deref();

    // Changed to include unsaved collections so the frontend dropdown can show them
    let collections = collection_repo::list_for_corridor(pool, game_id, is_safe, true).await?;

    let mut summaries = Vec::with_capacity(collections.len());
    for c in collections {
        summaries.push(collection_repo::to_summary(&c, active_id, undo_id));
    }

    Ok(summaries)
}

pub async fn create_collection(
    pool: &SqlitePool,
    input: CreateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    let is_safe_i32 = if input.is_safe { 1 } else { 0 };
    let id = uuid::Uuid::new_v4().to_string();
    let mods_path = load_game_mods_path(pool, &input.game_id).await?;
    let save_mode = input.save_mode.unwrap_or({
        if input.source_collection_id.is_some() {
            CreateCollectionMode::CloneSnapshot
        } else {
            CreateCollectionMode::SaveCurrentState
        }
    });
    let (persisted_mods, persisted_objects, roots, projected_state) = match save_mode {
        CreateCollectionMode::CloneSnapshot => {
            let Some(source_collection_id) = input.source_collection_id.as_deref() else {
                return Err(CollectionError::Validation(
                    "Clone snapshot requires a source collection".to_string(),
                ));
            };
            let source = collection_repo::get_by_id(pool, source_collection_id)
                .await?
                .ok_or_else(|| CollectionError::NotFound {
                    id: source_collection_id.to_string(),
                })?;
            if source.game_id != input.game_id || source.is_safe != input.is_safe {
                return Err(CollectionError::Validation(
                    "Snapshot source does not belong to the active corridor".to_string(),
                ));
            }

            let snapshot =
                load_projected_collection_state(pool, &source, mods_path.as_deref()).await?;
            let (mods, objects, roots) =
                collection_members_from_projected_state(&id, input.is_safe, &snapshot);
            (mods, objects, roots, snapshot)
        }
        CreateCollectionMode::SaveCurrentState => {
            if input.source_collection_id.is_some() {
                return Err(CollectionError::Validation(
                    "Save current state cannot use a source collection".to_string(),
                ));
            }

            let (mods, objects) =
                load_live_corridor_state(pool, &input.game_id, input.is_safe).await?;
            if mods.is_empty() {
                return Err(CollectionError::Validation(
                    "A collection must contain at least 1 active mod".to_string(),
                ));
            }

            let persisted_mods: Vec<CollectionMod> = mods
                .iter()
                .map(|entry| CollectionMod {
                    collection_id: id.clone(),
                    ..entry.clone()
                })
                .collect();
            let persisted_objects: Vec<CollectionObject> = objects
                .iter()
                .map(|entry| CollectionObject {
                    collection_id: id.clone(),
                    ..entry.clone()
                })
                .collect();
            let projected_state = build_projected_state_from_members(
                &persisted_mods,
                &persisted_objects,
                mods_path.as_deref(),
            );
            let roots = projected_state_service::roots_from_projected_state(
                &id,
                input.is_safe,
                &projected_state,
            );
            (persisted_mods, persisted_objects, roots, projected_state)
        }
    };

    let signature = projected_state_service::signature_for_projected_state(&projected_state);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state);

    // 3. Save to DB
    collection_repo::create(pool, &id, &input.game_id, &input.name, input.is_safe, false).await?;
    collection_repo::replace_all_state(
        pool,
        &id,
        &persisted_mods,
        &persisted_objects,
        &roots,
        Some(&signature),
        snapshot_json.as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await?;

    if matches!(save_mode, CreateCollectionMode::SaveCurrentState) {
        corridor_repo::update_pointers(pool, &input.game_id, input.is_safe, Some(&id), None)
            .await
            .map_err(CollectionError::Corridor)?;

        sqlx::query("DELETE FROM collections WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1")
            .bind(&input.game_id)
            .bind(is_safe_i32)
            .execute(pool)
            .await?;
    }

    // Return summary
    let collection = collection_repo::get_by_id(pool, &id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.clone() })?;

    let active_collection_id = if matches!(save_mode, CreateCollectionMode::SaveCurrentState) {
        Some(id.clone())
    } else {
        corridor_repo::get(pool, &input.game_id, input.is_safe)
            .await
            .map_err(CollectionError::Corridor)?
            .and_then(|state| state.active_collection_id)
    };

    Ok(collection_repo::to_summary(
        &collection,
        active_collection_id.as_deref(),
        None,
    ))
}

pub async fn handle_dirty_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CollectionSummary, CollectionError> {
    persist_corridor_runtime_snapshot(pool, game_id, is_safe).await
}

pub(crate) async fn persist_corridor_runtime_snapshot(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CollectionSummary, CollectionError> {
    let mods_path = load_game_mods_path(pool, game_id).await?;
    let timestamp_name = chrono::Local::now().format("%Y%m%d%H%M").to_string();
    let (mods, objects) = load_live_corridor_state(pool, game_id, is_safe).await?;
    let mut tx = pool.begin().await?;
    let collection_id = write_corridor_runtime_snapshot_tx(
        &mut tx,
        game_id,
        is_safe,
        mods_path.as_deref(),
        &timestamp_name,
        &mods,
        &objects,
    )
    .await?;
    tx.commit().await?;

    let collection = collection_repo::get_by_id(pool, &collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.clone(),
        })?;

    Ok(collection_repo::to_summary(
        &collection,
        Some(&collection_id),
        None,
    ))
}

async fn write_corridor_runtime_snapshot_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    is_safe: bool,
    mods_path: Option<&str>,
    timestamp_name: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
) -> Result<String, CollectionError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    let unsaved_ids: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM collections
        WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1
        ORDER BY updated_at DESC, id ASC
        "#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_all(&mut *conn)
    .await?;
    let collection_id = unsaved_ids
        .first()
        .cloned()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let name_key = crate::services::path_key::collection_name_key(timestamp_name);

    if unsaved_ids.is_empty() {
        sqlx::query(
            r#"
            INSERT INTO collections (id, game_id, name, name_key, is_safe, is_unsaved, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&collection_id)
        .bind(game_id)
        .bind(timestamp_name)
        .bind(&name_key)
        .bind(is_safe_i32)
        .execute(&mut *conn)
        .await?;
    } else {
        sqlx::query(
            "UPDATE collections SET name = ?, name_key = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(timestamp_name)
        .bind(&name_key)
        .bind(&collection_id)
        .execute(&mut *conn)
        .await?;

        for duplicate_id in unsaved_ids.iter().skip(1) {
            collection_repo::delete_tx(&mut *conn, duplicate_id).await?;
        }
    }

    let persisted_mods: Vec<CollectionMod> = mods
        .iter()
        .map(|entry| CollectionMod {
            collection_id: collection_id.clone(),
            ..entry.clone()
        })
        .collect();
    let persisted_objects: Vec<CollectionObject> = objects
        .iter()
        .map(|entry| CollectionObject {
            collection_id: collection_id.clone(),
            ..entry.clone()
        })
        .collect();
    let projected_state =
        build_projected_state_from_members(&persisted_mods, &persisted_objects, mods_path);
    let roots = projected_state_service::roots_from_projected_state(
        &collection_id,
        is_safe,
        &projected_state,
    );
    let signature = projected_state_service::signature_for_projected_state(&projected_state);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state);
    collection_repo::replace_all_state_tx(
        &mut *conn,
        &collection_id,
        &persisted_mods,
        &persisted_objects,
        &roots,
        Some(&signature),
        snapshot_json.as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await?;
    corridor_repo::update_pointers_tx(&mut *conn, game_id, is_safe, Some(&collection_id), None)
        .await?;
    Ok(collection_id)
}

pub async fn get_collection_preview(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    mods_path: Option<&str>,
) -> Result<CollectionPreview, CollectionError> {
    let collection = collection_repo::get_by_id(pool, collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.to_string(),
        })?;
    if collection.game_id != game_id {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to game '{}'",
            collection_id, game_id
        )));
    }

    let projected_state = load_projected_collection_state(pool, &collection, mods_path).await?;
    let mods = projected_state_service::mods_from_projected_state(collection_id, &projected_state);
    let objects =
        projected_state_service::objects_from_projected_state(collection_id, &projected_state);
    let roots = projected_state_service::roots_from_projected_state(
        collection_id,
        collection.is_safe,
        &projected_state,
    );

    let corridor_snapshot = crate::services::corridor_service::get_corridor_state(
        pool,
        &collection.game_id,
        collection.is_safe,
    )
    .await
    .map_err(CollectionError::Corridor)?;
    let active_id = corridor_snapshot.active_collection_id.as_deref();
    let undo_id = corridor_snapshot.undo_collection_id.as_deref();

    // Build unified members list for frontend convenience
    let mut members = Vec::new();
    for m in &mods {
        members.push(crate::domain::collection::CollectionMember::Mod(m.clone()));
    }
    for o in &objects {
        members.push(crate::domain::collection::CollectionMember::Object(
            o.clone(),
        ));
    }
    for r in &roots {
        members.push(crate::domain::collection::CollectionMember::Root(r.clone()));
    }
    let tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&projected_state);

    Ok(CollectionPreview {
        collection: collection_repo::to_summary(&collection, active_id, undo_id),
        members,
        mods,
        objects,
        roots,
        tree_nodes,
        projected_state,
    })
}

pub fn compute_signature(mods: &[CollectionMod], objects: &[CollectionObject]) -> String {
    let projected_state = projected_state_service::build_projected_state(mods, objects, None);
    projected_state_service::signature_for_projected_state(&projected_state)
}

pub async fn delete_collection(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    let collection = collection_repo::get_by_id(pool, id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.to_string() })?;
    let mods_path = load_game_mods_path(pool, &collection.game_id).await?;
    let timestamp_name = chrono::Local::now().format("%Y%m%d%H%M").to_string();
    let live_state =
        load_live_corridor_state(pool, &collection.game_id, collection.is_safe).await?;

    let corridor = corridor_repo::get(pool, &collection.game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?;

    let was_active = corridor
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
        == Some(id);
    let fallback_active = if was_active {
        collection_repo::find_unsaved_for_corridor(
            pool,
            &collection.game_id,
            collection.is_safe,
            Some(id),
        )
        .await?
        .map(|unsaved| unsaved.id)
    } else {
        None
    };

    let mut tx = pool.begin().await?;
    corridor_repo::clear_collection_references_tx(&mut tx, id)
        .await
        .map_err(CollectionError::Corridor)?;
    collection_repo::delete_tx(&mut tx, id).await?;

    if was_active {
        if let Some(fallback_active_id) = fallback_active.as_deref() {
            corridor_repo::update_pointers_tx(
                &mut tx,
                &collection.game_id,
                collection.is_safe,
                Some(fallback_active_id),
                None,
            )
            .await
            .map_err(CollectionError::Corridor)?;
        } else {
            write_corridor_runtime_snapshot_tx(
                &mut tx,
                &collection.game_id,
                collection.is_safe,
                mods_path.as_deref(),
                &timestamp_name,
                &live_state.0,
                &live_state.1,
            )
            .await?;
        }
    }
    tx.commit().await?;

    Ok(())
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    if let Some(ref name) = input.name {
        sqlx::query("UPDATE collections SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(name)
            .bind(&input.id)
            .execute(pool)
            .await?;
    }
    let collection = collection_repo::get_by_id(pool, &input.id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: input.id.clone(),
        })?;
    let _ = load_projected_collection_state(pool, &collection, None).await?;
    let collection = collection_repo::get_by_id(pool, &input.id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: input.id.clone(),
        })?;

    Ok(collection_repo::to_summary(&collection, None, None))
}

pub async fn replace_collection_with_current_state(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
) -> Result<CollectionSummary, CollectionError> {
    let collection = collection_repo::get_by_id(pool, collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.to_string(),
        })?;

    if collection.game_id != game_id {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to game '{}'",
            collection_id, game_id
        )));
    }
    if collection.is_unsaved {
        return Err(CollectionError::Validation(
            "Cannot replace an unsaved collection snapshot".to_string(),
        ));
    }

    let mods_path = load_game_mods_path(pool, game_id).await?;
    let (mods, objects) = load_live_corridor_state(pool, game_id, collection.is_safe).await?;
    if mods.is_empty() {
        return Err(CollectionError::Validation(
            "A collection must contain at least 1 active mod".to_string(),
        ));
    }

    let persisted_mods: Vec<CollectionMod> = mods
        .iter()
        .map(|entry| CollectionMod {
            collection_id: collection.id.clone(),
            ..entry.clone()
        })
        .collect();
    let persisted_objects: Vec<CollectionObject> = objects
        .iter()
        .map(|entry| CollectionObject {
            collection_id: collection.id.clone(),
            ..entry.clone()
        })
        .collect();
    let projected_state = build_projected_state_from_members(
        &persisted_mods,
        &persisted_objects,
        mods_path.as_deref(),
    );
    let roots = projected_state_service::roots_from_projected_state(
        &collection.id,
        collection.is_safe,
        &projected_state,
    );
    let signature = projected_state_service::signature_for_projected_state(&projected_state);
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state);

    collection_repo::replace_all_state(
        pool,
        &collection.id,
        &persisted_mods,
        &persisted_objects,
        &roots,
        Some(&signature),
        snapshot_json.as_deref(),
        projected_state.summary.active_root_count as i32,
    )
    .await?;

    let updated = collection_repo::get_by_id(pool, &collection.id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection.id.clone(),
        })?;
    let corridor = corridor_repo::get(pool, game_id, collection.is_safe)
        .await
        .map_err(CollectionError::Corridor)?;
    let active_id = corridor
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref());
    let undo_id = corridor
        .as_ref()
        .and_then(|state| state.undo_collection_id.as_deref());

    Ok(collection_repo::to_summary(&updated, active_id, undo_id))
}

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
            let pattern = format!("{}%", old_sep);
            let rows = sqlx::query(
                r#"
                SELECT cm.collection_id, c.name, cm.mod_path
                FROM collection_mods cm
                INNER JOIN collections c ON c.id = cm.collection_id
                WHERE cm.mod_path LIKE ?
                "#,
            )
            .bind(&pattern)
            .fetch_all(&mut *conn)
            .await?;

            for r in rows {
                use sqlx::Row;
                let col_id: String = r.get("collection_id");
                let collection_name: String = r.get("name");
                let old_path: String = r.get("mod_path");
                let new_path = old_path.replacen(&old_sep, &new_sep, 1);

                sqlx::query(
                    r#"
                    UPDATE collection_mods
                    SET
                        mod_path = ?,
                        mod_path_key = ?,
                        preview_path = CASE
                            WHEN preview_path = ? THEN ?
                            WHEN preview_path LIKE ? THEN REPLACE(preview_path, ?, ?)
                            ELSE preview_path
                        END
                    WHERE collection_id = ? AND mod_path = ?
                    "#,
                )
                .bind(&new_path)
                .bind(crate::services::path_key::folder_path_key(&new_path, None))
                .bind(&old_path)
                .bind(&new_path)
                .bind(format!("{}%", old_sep))
                .bind(&old_sep)
                .bind(&new_sep)
                .bind(&col_id)
                .bind(&old_path)
                .execute(&mut *conn)
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
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT c.id, c.name
        FROM collections c
        INNER JOIN collection_mods cm ON cm.collection_id = c.id
        WHERE cm.mod_path = ?
        ORDER BY c.name ASC, c.id ASC
        "#,
    )
    .bind(mod_path)
    .fetch_all(&mut *conn)
    .await?;

    let references = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            Ok(CollectionReferenceRow {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok(references)
}

async fn recompute_signature_tx(
    conn: &mut sqlx::SqliteConnection,
    collection_id: &str,
) -> Result<(), CollectionError> {
    let collection_context = sqlx::query(
        r#"
        SELECT c.is_safe, g.mods_path
        FROM collections c
        LEFT JOIN games g ON g.id = c.game_id
        WHERE c.id = ?
        "#,
    )
    .bind(collection_id)
    .fetch_one(&mut *conn)
    .await?;
    let is_safe: i32 = {
        use sqlx::Row;
        collection_context.try_get("is_safe")?
    };
    let mods_path: Option<String> = {
        use sqlx::Row;
        collection_context.try_get("mods_path").unwrap_or(None)
    };
    let rows = sqlx::query(
        "SELECT collection_id, mod_id, mod_path, mod_path_key, object_id, preview_path, node_type, warnings_json FROM collection_mods WHERE collection_id = ?"
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;

    let mut mods = Vec::new();
    for r in rows {
        use sqlx::Row;
        mods.push(crate::domain::collection::CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: r.try_get("collection_id").unwrap_or_default(),
            mod_id: r.try_get("mod_id").unwrap_or_default(),
            mod_path: r.try_get("mod_path").unwrap_or_default(),
            mod_path_key: r.try_get("mod_path_key").unwrap_or_default(),
            object_id: r.try_get("object_id").unwrap_or_default(),
            display_name: None,
            preview_path: r.try_get("preview_path").unwrap_or_default(),
            node_type: r.try_get("node_type").unwrap_or_default(),
            warnings: parse_warnings_json(r.try_get("warnings_json").ok()),
            is_enabled: true,
        });
    }

    let object_rows = sqlx::query(
        r#"SELECT collection_id, object_id, is_enabled
           FROM collection_objects
           WHERE collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;
    let mut objects = Vec::with_capacity(object_rows.len());
    for row in object_rows {
        use sqlx::Row;
        objects.push(crate::domain::collection::CollectionObject {
            kind: crate::domain::collection::MemberKind::Object,
            collection_id: row.try_get("collection_id").unwrap_or_default(),
            object_id: row.try_get("object_id").unwrap_or_default(),
            is_enabled: row.try_get::<i32, _>("is_enabled").unwrap_or(1) != 0,
            display_name: None,
            path_key: None,
        });
    }
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

pub async fn preview_apply(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    is_safe: bool,
    mods_path: Option<&str>,
) -> Result<ApplyPreview, CollectionError> {
    let collection = collection_repo::get_by_id(pool, collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.to_string(),
        })?;
    if collection.game_id != game_id {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to game '{}'",
            collection_id, game_id
        )));
    }
    if collection.is_safe != is_safe {
        return Err(CollectionError::Validation(format!(
            "Collection '{}' does not belong to requested corridor",
            collection_id
        )));
    }
    if mods_path.is_some_and(|path| {
        let root = std::path::Path::new(path);
        !root.exists() || !root.is_dir()
    }) {
        return Err(CollectionError::Corridor(
            crate::domain::errors::CorridorError::NoModsPath {
                game_id: game_id.to_string(),
            },
        ));
    }

    let corridor_snapshot =
        crate::services::corridor_service::get_corridor_state(pool, game_id, is_safe)
            .await
            .map_err(CollectionError::Corridor)?;
    let target_state = load_projected_collection_state(pool, &collection, mods_path).await?;
    let target_mods =
        projected_state_service::mods_from_projected_state(collection_id, &target_state);
    let target_objects =
        projected_state_service::objects_from_projected_state(collection_id, &target_state);

    Ok(ApplyPreview {
        collection_name: collection.name,
        current_snapshot: Some(corridor_snapshot.current_signature),
        current_mods: corridor_snapshot.current_mods.clone(),
        current_objects: corridor_snapshot.current_objects.clone(),
        current_tree_nodes: corridor_snapshot.current_tree_nodes.clone(),
        target_mods: target_mods.clone(),
        target_objects: target_objects.clone(),
        target_tree_nodes: projected_state_service::build_preview_tree_from_projected_state(
            &target_state,
        ),
        current_state_name: corridor_snapshot.active_collection_name.clone(),
        current_state_is_unsaved: corridor_snapshot.is_dirty
            || corridor_snapshot.active_collection_is_unsaved,
        current_projected_state: corridor_snapshot.projected_state.clone(),
        target_projected_state: target_state,
    })
}

pub(crate) async fn load_live_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<(Vec<CollectionMod>, Vec<CollectionObject>), CollectionError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    let mods_path = load_game_mods_path(pool, game_id).await?;
    let current_objects: Vec<CollectionObject> = sqlx::query_as(
        r#"
        SELECT
            'object' as kind,
            '' as collection_id,
            id as object_id,
            1 as is_enabled,
            name as display_name,
            folder_path as path_key
        FROM objects
        WHERE game_id = ?
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    let current_objects: Vec<CollectionObject> = current_objects
        .into_iter()
        .map(|object| CollectionObject {
            is_enabled: is_object_enabled(object.path_key.as_deref()),
            ..object
        })
        .collect();
    let current_mod_rows = sqlx::query(
        r#"
        SELECT
            id as mod_id,
            folder_path as mod_path,
            folder_path_key as mod_path_key,
            object_id,
            actual_name as display_name
        FROM mods
        WHERE game_id = ? AND is_safe = ? AND status = 1
        "#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_all(pool)
    .await?;

    let mut current_mods = Vec::with_capacity(current_mod_rows.len());
    for row in current_mod_rows {
        use sqlx::Row;

        let mod_id: String = row.get("mod_id");
        let mod_path: String = row.get("mod_path");
        let mod_path_key: String = row.get("mod_path_key");
        let object_id: String = row.get("object_id");
        let display_name: String = row.get("display_name");
        let preview_object = current_objects
            .iter()
            .find(|object| object.object_id == object_id);
        let preview_seed = CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: String::new(),
            mod_id: Some(mod_id.clone()),
            mod_path: mod_path.clone(),
            mod_path_key: Some(mod_path_key.clone()),
            object_id: object_id.clone(),
            display_name: Some(display_name.clone()),
            preview_path: None,
            node_type: None,
            warnings: Vec::new(),
            is_enabled: true,
        };
        let preview_metadata =
            resolve_preview_terminal_metadata(preview_object, &preview_seed, mods_path.as_deref());

        current_mods.push(CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: String::new(),
            mod_id: Some(mod_id),
            mod_path,
            mod_path_key: Some(mod_path_key),
            object_id,
            display_name: Some(display_name),
            preview_path: preview_metadata.preview_path,
            node_type: preview_metadata.node_type,
            warnings: preview_metadata.warnings,
            is_enabled: true,
        });
    }

    Ok((current_mods, current_objects))
}

async fn load_game_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, CollectionError> {
    Ok(
        sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
            .bind(game_id)
            .fetch_optional(pool)
            .await?,
    )
}

fn parse_warnings_json(raw: Option<String>) -> Vec<String> {
    let Some(raw_json) = raw else {
        return Vec::new();
    };

    serde_json::from_str::<Vec<String>>(&raw_json).unwrap_or_default()
}

pub struct ApplyCollectionRequest<'a> {
    pub pool: &'a SqlitePool,
    pub game_id: &'a str,
    pub collection_id: &'a str,
    pub is_safe: bool,
    pub mods_path: std::path::PathBuf,
    pub suppressor: std::sync::Arc<crate::services::scanner::watcher::WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: crate::services::config::AppSettings,
}

pub async fn apply_collection(
    request: ApplyCollectionRequest<'_>,
) -> Result<ApplyResult, CollectionError> {
    let mut ctx = crate::pipeline::apply_pipeline::ApplyContext::new(
        crate::pipeline::apply_pipeline::ApplyContextInput {
            pool: request.pool.clone(),
            game_id: request.game_id.to_string(),
            collection_id: request.collection_id.to_string(),
            is_safe: request.is_safe,
            mods_path: request.mods_path,
            suppressor: request.suppressor,
            ignore_missing: request.ignore_missing,
            settings: request.settings,
        },
    );

    crate::pipeline::apply_pipeline::execute(&mut ctx).await
}

pub async fn apply_collection_internal(
    request: ApplyCollectionRequest<'_>,
) -> Result<ApplyResult, CollectionError> {
    let mut ctx = crate::pipeline::apply_pipeline::ApplyContext::new(
        crate::pipeline::apply_pipeline::ApplyContextInput {
            pool: request.pool.clone(),
            game_id: request.game_id.to_string(),
            collection_id: request.collection_id.to_string(),
            is_safe: request.is_safe,
            mods_path: request.mods_path,
            suppressor: request.suppressor,
            ignore_missing: request.ignore_missing,
            settings: request.settings,
        },
    )
    .without_task();

    crate::pipeline::apply_pipeline::execute(&mut ctx).await
}

#[cfg(test)]
mod tests;
