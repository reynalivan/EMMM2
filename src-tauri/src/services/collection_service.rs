use crate::domain::collection::{
    ApplyPreview, ApplyResult, CollectionMod, CollectionObject, CollectionPreview, CollectionRoot,
    CollectionSummary, CreateCollectionInput, UpdateCollectionInput,
};
use crate::domain::errors::CollectionError;
use crate::repo::{collection_repo, corridor_repo};
use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// collection_service — Business logic for collections
// ---------------------------------------------------------------------------

/// List all named collections for a game in the current corridor.
pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Vec<CollectionSummary>, CollectionError> {
    let corridor = corridor_repo::get(pool, game_id, is_safe)
        .await
        .map_err(CollectionError::Corridor)?;

    let active_id = corridor
        .as_ref()
        .and_then(|c| c.active_collection_id.as_deref());
    let undo_id = corridor
        .as_ref()
        .and_then(|c| c.undo_collection_id.as_deref());

    // Changed to include unsaved collections so the frontend dropdown can show them
    let collections = collection_repo::list_for_corridor(pool, game_id, is_safe, true).await?;

    let mut summaries = Vec::new();
    for c in collections {
        // Fetch member count (mods + objects)
        let mod_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_mods WHERE collection_id = ?")
            .bind(&c.id)
            .fetch_one(pool)
            .await?;
        let obj_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_objects WHERE collection_id = ?")
            .bind(&c.id)
            .fetch_one(pool)
            .await?;
        
        summaries.push(collection_repo::to_summary(&c, active_id, undo_id, mod_count + obj_count));
    }

    Ok(summaries)
}

pub async fn create_collection(
    pool: &SqlitePool,
    input: CreateCollectionInput,
) -> Result<CollectionSummary, CollectionError> {
    let id = uuid::Uuid::new_v4().to_string();
    let is_safe_i32 = if input.is_safe { 1i32 } else { 0i32 };

    // 1. Snapshot currently-enabled objects and mods
    let enabled_objects: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, name, folder_path FROM objects WHERE game_id = ?")
            .bind(&input.game_id)
            .fetch_all(pool)
            .await?;

    let enabled_mods: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, object_id, folder_path, folder_path_key, actual_name FROM mods WHERE game_id = ? AND is_safe = ? AND status = 1"
    )
    .bind(&input.game_id)
    .bind(is_safe_i32)
    .fetch_all(pool)
    .await
    ?;

    if enabled_mods.is_empty() {
        return Err(CollectionError::Validation(
            "A collection must contain at least 1 active mod".to_string(),
        ));
    }

    let mut mods = Vec::new();
    let mut objects = Vec::new();
    let mut roots = Vec::new();

    for (oid, name, path) in enabled_objects {
        objects.push(CollectionObject {
            kind: crate::domain::collection::MemberKind::Object,
            collection_id: id.clone(),
            object_id: oid.clone(),
            is_enabled: true,
            display_name: Some(name.clone()),
            path_key: Some(oid.clone()),
        });
        roots.push(CollectionRoot {
            kind: crate::domain::collection::MemberKind::Root,
            collection_id: id.clone(),
            root_path: path,
            root_path_key: oid.clone(),
            display_name: name,
            display_name_key: oid.clone(),
            object_id: Some(oid),
            object_name: None,
            object_type: None,
            root_kind: "object".to_string(),
            is_safe: input.is_safe,
            is_enabled: true,
            thumbnail_hint: None,
            corridor_source: None,
        });
    }

    for (mid, oid, path, key, actual_name) in enabled_mods {
        mods.push(CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: id.clone(),
            mod_id: Some(mid),
            mod_path: path,
            mod_path_key: Some(key),
            object_id: oid,
            display_name: Some(actual_name),
            is_enabled: true,
        });
    }

    // 2. Compute signature
    let signature = compute_signature(&mods);

    // 3. Save to DB
    collection_repo::create(pool, &id, &input.game_id, &input.name, input.is_safe, false).await?;
    collection_repo::replace_all_state(pool, &id, &mods, &objects, &roots, Some(&signature), None)
        .await?;

    // 4. Update corridor pointers
    corridor_repo::update_pointers(pool, &input.game_id, input.is_safe, Some(&id), None)
        .await
        .map_err(CollectionError::Corridor)?;

    // 5. Cleanup any existing unsaved collection for this corridor
    sqlx::query("DELETE FROM collections WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1")
        .bind(&input.game_id)
        .bind(is_safe_i32)
        .execute(pool)
        .await?;

    // Return summary
    let collection = collection_repo::get_by_id(pool, &id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.clone() })?;

    let mod_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_mods WHERE collection_id = ?").bind(&id).fetch_one(pool).await?;
    let obj_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_objects WHERE collection_id = ?").bind(&id).fetch_one(pool).await?;

    Ok(collection_repo::to_summary(
        &collection,
        Some(&id),
        None,
        mod_count + obj_count,
    ))
}

pub async fn handle_dirty_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CollectionSummary, CollectionError> {
    // Phase 4.3 Refinement: Only transition to "Unsaved" if the active mods set has changed.
    // Toggling/Deleting disabled mods or renaming auto-healed mods should NOT trigger dirty state.
    let snapshot = crate::repo::corridor_repo::get_snapshot(pool, game_id, is_safe).await?;
    if let Some(ref active_id) = snapshot.active_collection_id {
        if let Ok(Some(coll)) = crate::repo::collection_repo::get_by_id(pool, active_id).await {
            let target_sig = coll.signature.as_deref().unwrap_or_default();
            if target_sig == snapshot.current_signature {
                // Disk matches Collection — no need for "Unsaved" transition
                let mod_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_mods WHERE collection_id = ?").bind(active_id).fetch_one(pool).await?;
                let obj_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_objects WHERE collection_id = ?").bind(active_id).fetch_one(pool).await?;

                return Ok(crate::repo::collection_repo::to_summary(
                    &coll,
                    Some(active_id),
                    None,
                    mod_count + obj_count,
                ));
            }
        }
    }

    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    let timestamp_name = chrono::Local::now().format("%Y%m%d%H%M").to_string();

    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM collections WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1",
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_optional(pool)
    .await?;

    let collection_id = if let Some(id) = existing {
        sqlx::query("UPDATE collections SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&timestamp_name)
            .bind(&id)
            .execute(pool)
            .await?;
        id
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        collection_repo::create(pool, &id, game_id, &timestamp_name, is_safe, true).await?;
        id
    };

    let enabled_mods: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, object_id, folder_path, folder_path_key, actual_name FROM mods WHERE game_id = ? AND is_safe = ? AND status = 1"
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_all(pool)
    .await
    ?;

    let enabled_objects: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, name, folder_path FROM objects WHERE game_id = ?")
            .bind(game_id)
            .fetch_all(pool)
            .await?;

    let mut mods = Vec::with_capacity(enabled_mods.len());
    let mut objects = Vec::with_capacity(enabled_objects.len());
    let mut roots = Vec::with_capacity(enabled_objects.len());

    for (mid, oid, path, key, actual_name) in enabled_mods {
        mods.push(CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: collection_id.clone(),
            mod_id: Some(mid),
            mod_path: path,
            mod_path_key: Some(key),
            object_id: oid,
            display_name: Some(actual_name),
            is_enabled: true,
        });
    }

    for (oid, name, path) in enabled_objects {
        objects.push(CollectionObject {
            kind: crate::domain::collection::MemberKind::Object,
            collection_id: collection_id.clone(),
            object_id: oid.clone(),
            is_enabled: true,
            display_name: Some(name.clone()),
            path_key: Some(oid.clone()),
        });
        roots.push(CollectionRoot {
            kind: crate::domain::collection::MemberKind::Root,
            collection_id: collection_id.clone(),
            root_path: path,
            root_path_key: oid.clone(),
            display_name: name,
            display_name_key: oid.clone(),
            object_id: Some(oid),
            object_name: None,
            object_type: None,
            root_kind: "object".to_string(),
            is_safe,
            is_enabled: true,
            thumbnail_hint: None,
            corridor_source: None,
        });
    }

    let signature = compute_signature(&mods);
    collection_repo::replace_all_state(
        pool,
        &collection_id,
        &mods,
        &objects,
        &roots,
        Some(&signature),
        None,
    )
    .await?;

    corridor_repo::update_pointers(pool, game_id, is_safe, Some(&collection_id), None)
        .await
        .map_err(CollectionError::Corridor)?;

    let collection = collection_repo::get_by_id(pool, &collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.clone(),
        })?;

    let mod_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_mods WHERE collection_id = ?").bind(&collection_id).fetch_one(pool).await?;
    let obj_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_objects WHERE collection_id = ?").bind(&collection_id).fetch_one(pool).await?;

    Ok(collection_repo::to_summary(
        &collection,
        Some(&collection_id),
        None,
        mod_count + obj_count,
    ))
}

pub async fn get_collection_preview(
    pool: &SqlitePool,
    collection_id: &str,
    _mods_path: Option<&str>,
) -> Result<CollectionPreview, CollectionError> {
    let collection = collection_repo::get_by_id(pool, collection_id)
        .await?
        .ok_or_else(|| CollectionError::NotFound {
            id: collection_id.to_string(),
        })?;

    let mods = collection_repo::get_mods(pool, collection_id).await?;
    let objects = collection_repo::get_objects(pool, collection_id).await?;
    let roots = collection_repo::get_roots(pool, collection_id).await?;

    let corridor = corridor_repo::get(pool, &collection.game_id, collection.is_safe)
        .await?
        .ok_or_else(|| CollectionError::Validation("Corridor state not found".to_string()))?;

    let active_id = corridor.active_collection_id.as_deref();
    let undo_id = corridor.undo_collection_id.as_deref();

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

    let mod_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_mods WHERE collection_id = ?").bind(collection_id).fetch_one(pool).await?;
    let obj_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_objects WHERE collection_id = ?").bind(collection_id).fetch_one(pool).await?;

    Ok(CollectionPreview {
        collection: collection_repo::to_summary(&collection, active_id, undo_id, mod_count + obj_count),
        members,
        mods,
        objects,
        roots,
    })
}

pub fn compute_signature(mods: &[CollectionMod]) -> String {
    let mut ids: Vec<String> = mods.iter().filter_map(|m| m.mod_id.clone()).collect();
    ids.sort();
    blake3::hash(ids.join("\n").as_bytes()).to_hex().to_string()
}

pub async fn delete_collection(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    collection_repo::delete(pool, id).await?;
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

    let mod_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_mods WHERE collection_id = ?").bind(&input.id).fetch_one(pool).await?;
    let obj_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_objects WHERE collection_id = ?").bind(&input.id).fetch_one(pool).await?;

    Ok(collection_repo::to_summary(
        &collection,
        None,
        None,
        mod_count + obj_count,
    ))
}

pub async fn handle_mod_moved_or_renamed(
    pool: &SqlitePool,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<u64, CollectionError> {
    let mut tx = pool.begin().await?;
    let count =
        handle_mod_moved_or_renamed_tx(&mut *tx, old_mod_path, new_mod_path, new_object_id).await?;
    tx.commit().await?;
    Ok(count)
}

pub async fn handle_mod_moved_or_renamed_tx(
    conn: &mut sqlx::SqliteConnection,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<u64, CollectionError> {
    let count =
        collection_repo::update_member_paths(&mut *conn, old_mod_path, new_mod_path, new_object_id)
            .await?;

    if count > 0 {
        // Find all collections that now contain this new path
        let collection_ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT collection_id FROM collection_mods WHERE mod_path = ?",
        )
        .bind(new_mod_path)
        .fetch_all(&mut *conn)
        .await?;

        // Recompute and update signatures
        for id in collection_ids {
            recompute_signature_tx(&mut *conn, &id).await?;
        }
    }

    Ok(count)
}

pub async fn handle_object_renamed_tx(
    conn: &mut sqlx::SqliteConnection,
    old_object_folder: &str,
    new_object_folder: &str,
) -> Result<(), CollectionError> {
    let mut affected_collections = std::collections::HashSet::new();

    for (old_sep, new_sep) in [
        (
            format!("{}\\", old_object_folder),
            format!("{}\\", new_object_folder),
        ),
        (
            format!("{}/", old_object_folder),
            format!("{}/", new_object_folder),
        ),
    ] {
        let pattern = format!("{}%", old_sep);
        let rows = sqlx::query(
            "SELECT collection_id, mod_path FROM collection_mods WHERE mod_path LIKE ?",
        )
        .bind(&pattern)
        .fetch_all(&mut *conn)
        .await?;

        for r in rows {
            use sqlx::Row;
            let col_id: String = r.get("collection_id");
            let old_path: String = r.get("mod_path");
            let new_path = old_path.replacen(&old_sep, &new_sep, 1);

            sqlx::query(
                "UPDATE collection_mods SET mod_path = ? WHERE collection_id = ? AND mod_path = ?",
            )
            .bind(&new_path)
            .bind(&col_id)
            .bind(&old_path)
            .execute(&mut *conn)
            .await?;

            affected_collections.insert(col_id);
        }
    }

    for id in affected_collections {
        recompute_signature_tx(&mut *conn, &id).await?;
    }

    Ok(())
}

async fn recompute_signature_tx(
    conn: &mut sqlx::SqliteConnection,
    collection_id: &str,
) -> Result<(), CollectionError> {
    let rows = sqlx::query(
        "SELECT collection_id, mod_id, mod_path, mod_path_key, object_id FROM collection_mods WHERE collection_id = ?"
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
            is_enabled: true,
        });
    }

    let signature = compute_signature(&mods);
    sqlx::query(
        "UPDATE collections SET signature = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(signature)
    .bind(collection_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn preview_apply(
    _pool: &SqlitePool,
    _game_id: &str,
    _collection_id: &str,
    _is_safe: bool,
    _mods_path: Option<&str>,
) -> Result<ApplyPreview, CollectionError> {
    // Placeholder for now as this needs deeper refactor of how we preview diffs
    Err(CollectionError::Validation(
        "Preview not implemented for new schema yet".to_string(),
    ))
}

pub async fn apply_collection(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    is_safe: bool,
    mods_path: std::path::PathBuf,
    suppressor: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ignore_missing: bool,
    settings: crate::services::config::AppSettings,
) -> Result<ApplyResult, CollectionError> {
    let mut ctx = crate::pipeline::apply_pipeline::ApplyContext::new(
        pool.clone(),
        game_id.to_string(),
        collection_id.to_string(),
        is_safe,
        mods_path,
        suppressor,
        ignore_missing,
        settings,
    );

    crate::pipeline::apply_pipeline::execute(&mut ctx).await
}

pub async fn undo_collection(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    mods_path: std::path::PathBuf,
    suppressor: std::sync::Arc<std::sync::atomic::AtomicBool>,
    settings: crate::services::config::AppSettings,
) -> Result<ApplyResult, CollectionError> {
    let corridor = corridor_repo::get(pool, game_id, is_safe)
        .await
        .map_err(CollectionError::Corridor)?
        .ok_or_else(|| CollectionError::NoUndoAvailable)?;

    let undo_id = corridor
        .undo_collection_id
        .ok_or(CollectionError::NoUndoAvailable)?;

    apply_collection(
        pool, game_id, &undo_id, is_safe, mods_path, suppressor, true, settings,
    )
    .await
}
