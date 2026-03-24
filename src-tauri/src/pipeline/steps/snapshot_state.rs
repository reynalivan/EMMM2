use crate::domain::collection::{CollectionMod, CollectionObject, CollectionRoot, MemberKind};
use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::repo::collection_repo;

/// Step 5: Create an undo snapshot of the current state before applying changes.
pub async fn snapshot(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    if ctx.to_enable.is_empty() && ctx.to_disable.is_empty() {
        log::info!("apply_pipeline[snapshot]: no changes needed, skipping undo snapshot");
        return Ok(());
    }

    let snapshot_id = format!("undo_{}", uuid::Uuid::new_v4());

    // 1. Snapshot currently-enabled objects and mods
    let is_safe_i32 = if ctx.is_safe { 1i32 } else { 0i32 };

    let enabled_objects: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, folder_path FROM objects WHERE game_id = ? AND status = 1",
    )
    .bind(&ctx.game_id)
    .fetch_all(&ctx.pool)
    .await?;

    let enabled_mods: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, object_id, folder_path, folder_path_key, actual_name FROM mods WHERE game_id = ? AND is_safe = ? AND status = 1"
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .fetch_all(&ctx.pool)
    .await?;

    // Create the undo_snapshot collection
    let _snapshot = collection_repo::create(
        &ctx.pool,
        &snapshot_id,
        &ctx.game_id,
        "Undo Snapshot",
        ctx.is_safe,
        false, // persistent
    )
    .await?;

    let mut mods = Vec::new();
    let mut objects = Vec::new();
    let mut roots = Vec::new();

    for (oid, name, path) in enabled_objects {
        objects.push(CollectionObject {
            kind: MemberKind::Object,
            collection_id: snapshot_id.clone(),
            object_id: oid.clone(),
            is_enabled: true,
            display_name: Some(name.clone()),
            path_key: Some(oid.clone()),
        });
        roots.push(CollectionRoot {
            kind: MemberKind::Root,
            collection_id: snapshot_id.clone(),
            root_path: path.clone(),
            root_path_key: oid.clone(),
            display_name: name,
            display_name_key: oid.clone(),
            object_id: Some(oid),
            object_name: None,
            object_type: None,
            root_kind: "object".to_string(),
            is_safe: ctx.is_safe,
            is_enabled: true,
            thumbnail_hint: None,
            corridor_source: None,
        });
    }

    for (mid, oid, path, key, actual_name) in enabled_mods {
        mods.push(CollectionMod {
            kind: MemberKind::Mod,
            collection_id: snapshot_id.clone(),
            mod_id: Some(mid.clone()),
            mod_path: path.clone(),
            mod_path_key: Some(key.clone()),
            object_id: oid.clone(),
            display_name: Some(actual_name),
            is_enabled: true,
        });
        roots.push(CollectionRoot {
            kind: MemberKind::Root,
            collection_id: snapshot_id.clone(),
            root_path: path,
            root_path_key: key,
            display_name: "Mod".to_string(),
            display_name_key: mid,
            object_id: Some(oid),
            object_name: None,
            object_type: None,
            root_kind: "mod".to_string(),
            is_safe: ctx.is_safe,
            is_enabled: true,
            thumbnail_hint: None,
            corridor_source: None,
        });
    }

    // Compute signature
    let signature = crate::services::collection_service::compute_signature(&mods);

    collection_repo::replace_all_state(
        &ctx.pool,
        &snapshot_id,
        &mods,
        &objects,
        &roots,
        Some(&signature),
        None,
    )
    .await?;

    ctx.undo_snapshot_id = Some(snapshot_id.clone());

    log::info!(
        "apply_pipeline[snapshot]: created undo snapshot '{}' with {} mods and {} objects",
        snapshot_id,
        mods.len(),
        objects.len()
    );

    Ok(())
}
