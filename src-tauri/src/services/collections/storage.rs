use super::nested_walker;
use super::types::{
    Collection, CollectionDetails, CollectionPreviewMod, CreateCollectionInput,
    UpdateCollectionInput,
};
use crate::database::collection_repo;
use crate::database::game_repo;
use sqlx::SqlitePool;
use std::collections::HashSet;
use uuid::Uuid;

pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<Vec<Collection>, String> {
    collection_repo::list_collections(pool, game_id, safe_mode_enabled)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_collection(
    pool: &SqlitePool,
    input: CreateCollectionInput,
) -> Result<CollectionDetails, String> {
    let id = Uuid::new_v4().to_string();

    // Resolve mods_path BEFORE starting transaction to avoid connection starvation
    let mods_path = game_repo::get_mod_path(pool, &input.game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let name_trimmed = input.name.trim();

    let exists = collection_repo::check_collection_exists(
        &mut tx,
        &input.game_id,
        name_trimmed,
        input.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;

    if exists {
        return Err(format!(
            "A collection named '{}' already exists.",
            name_trimmed
        ));
    }

    collection_repo::insert_collection(
        &mut tx,
        &id,
        name_trimmed,
        &input.game_id,
        input.is_safe_context,
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut mod_ids = input.mod_ids;
    if input.auto_snapshot.unwrap_or(false) {
        mod_ids = collection_repo::get_enabled_mod_ids_for_corridor(
            &mut tx,
            &input.game_id,
            input.is_safe_context,
        )
        .await
        .map_err(|e| e.to_string())?;
    }

    let mod_ids = unique_mod_ids(mod_ids);

    let paths = collection_repo::get_mod_paths_for_ids(&mut tx, &mod_ids)
        .await
        .map_err(|e| e.to_string())?;

    for mod_id in &mod_ids {
        let mod_path = paths.get(mod_id).map(|p| p.as_str());
        collection_repo::insert_collection_item(&mut tx, &id, mod_id, mod_path)
            .await
            .map_err(|e| e.to_string())?;

        // Metadata Portability (US-8.3)
        if let Some(path) = mod_path {
            use crate::services::mods::info_json::{update_info_json, ModInfoUpdate};
            let update = ModInfoUpdate {
                preset_name_add: Some(vec![name_trimmed.to_string()]),
                ..Default::default()
            };
            let _ = update_info_json(std::path::Path::new(path), &update);
        }
    }

    // ── Nested mods: walk filesystem for mods inside ContainerFolders ─────

    let mut nested_count = 0;
    if let Some(ref mp) = mods_path {
        if let Ok(nested) = nested_walker::walk_nested_mods(mp) {
            // Filter by safe context if needed
            let filtered: Vec<_> = if input.is_safe_context {
                nested
                    .into_iter()
                    .filter(|n| n.is_enabled && n.is_safe)
                    .collect()
            } else {
                nested.into_iter().filter(|n| n.is_enabled).collect()
            };

            for nm in &filtered {
                collection_repo::insert_nested_collection_item(&mut tx, &id, &nm.folder_path)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            nested_count = filtered.len();
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(CollectionDetails {
        collection: Collection {
            id,
            name: input.name.trim().to_string(),
            game_id: input.game_id,
            is_safe_context: input.is_safe_context,
            member_count: mod_ids.len() + nested_count,
            is_last_unsaved: false,
        },
        mod_ids,
    })
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionDetails, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let old_name: String = collection_repo::get_collection_name(&mut tx, &input.id, &input.game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Collection not found")?;

    let mut new_name = old_name.clone();

    if let Some(name) = input.name.as_ref() {
        let name_trimmed = name.trim();
        new_name = name_trimmed.to_string();

        // Guard: check for duplicate name within same corridor
        if new_name != old_name {
            // Get current is_safe_context for scoping the uniqueness check
            let current_safe: bool = sqlx::query_scalar(
                "SELECT is_safe_context FROM collections WHERE id = ? AND game_id = ?",
            )
            .bind(&input.id)
            .bind(&input.game_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

            let effective_safe = input.is_safe_context.unwrap_or(current_safe);

            let exists = collection_repo::check_collection_exists(
                &mut tx,
                &input.game_id,
                name_trimmed,
                effective_safe,
            )
            .await
            .map_err(|e| e.to_string())?;

            if exists {
                return Err(format!(
                    "A collection named '{}' already exists.",
                    name_trimmed
                ));
            }

            let items = collection_repo::get_collection_items(&mut tx, &input.id)
                .await
                .map_err(|e| e.to_string())?;

            let paths: Vec<String> = items.into_iter().filter_map(|(_, p)| p).collect();
            for path in paths {
                use crate::services::mods::info_json::{update_info_json, ModInfoUpdate};
                let update = ModInfoUpdate {
                    preset_name_remove: Some(vec![old_name.clone()]),
                    preset_name_add: Some(vec![new_name.clone()]),
                    ..Default::default()
                };
                let _ = update_info_json(std::path::Path::new(&path), &update);
            }
        }

        collection_repo::update_collection_name(&mut tx, &input.id, &input.game_id, name_trimmed)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(safe) = input.is_safe_context {
        collection_repo::update_collection_safe_context(&mut tx, &input.id, &input.game_id, safe)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(mod_ids) = input.mod_ids.as_ref() {
        let unique = unique_mod_ids(mod_ids.clone());

        // 1. Get old members to diff
        let old_items = collection_repo::get_collection_items(&mut tx, &input.id)
            .await
            .unwrap_or_default();

        let old_ids: HashSet<String> = old_items.iter().map(|(id, _)| id.clone()).collect();
        let new_ids: HashSet<String> = unique.iter().cloned().collect();

        let removed_ids: HashSet<_> = old_ids.difference(&new_ids).collect();
        let added_ids: HashSet<_> = new_ids.difference(&old_ids).collect();

        // 2. Remove preset_name from removed members
        for (id, path) in &old_items {
            if removed_ids.contains(id) {
                if let Some(p) = path {
                    use crate::services::mods::info_json::{update_info_json, ModInfoUpdate};
                    let update = ModInfoUpdate {
                        preset_name_remove: Some(vec![new_name.clone()]),
                        ..Default::default()
                    };
                    let _ = update_info_json(std::path::Path::new(p), &update);
                }
            }
        }

        // 3. Delete old DB entries
        collection_repo::delete_collection_items(&mut tx, &input.id)
            .await
            .map_err(|e| e.to_string())?;

        // 4. Fetch paths for new members
        let added_ids_vec: Vec<String> = added_ids.iter().map(|s| s.to_string()).collect();
        let add_paths = collection_repo::get_mod_paths_for_ids(&mut tx, &added_ids_vec)
            .await
            .unwrap_or_default();

        // 5. Insert new members and add to info.json
        for mod_id in &unique {
            let mod_path = add_paths.get(mod_id).map(|s| s.as_str()).or_else(|| {
                // fallback to old path if not newly added
                old_items
                    .iter()
                    .find(|(id, _)| id == mod_id)
                    .and_then(|(_, p)| p.as_deref())
            });

            collection_repo::insert_collection_item(&mut tx, &input.id, mod_id, mod_path)
                .await
                .map_err(|e| e.to_string())?;

            if added_ids.contains(mod_id) {
                if let Some(p) = mod_path {
                    use crate::services::mods::info_json::{update_info_json, ModInfoUpdate};
                    let update = ModInfoUpdate {
                        preset_name_add: Some(vec![new_name.clone()]),
                        ..Default::default()
                    };
                    let _ = update_info_json(std::path::Path::new(p), &update);
                }
            }
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    get_collection(pool, &input.id, &input.game_id).await
}

pub async fn delete_collection(pool: &SqlitePool, id: &str, game_id: &str) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let name = collection_repo::get_collection_name(&mut tx, id, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Collection not found")?;

    let items = collection_repo::get_collection_items(&mut tx, id)
        .await
        .unwrap_or_default();

    collection_repo::delete_collection(&mut tx, id, game_id)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    for (_, maybe_path) in items {
        if let Some(path) = maybe_path {
            use crate::services::mods::info_json::{update_info_json, ModInfoUpdate};
            let update = ModInfoUpdate {
                preset_name_remove: Some(vec![name.clone()]),
                ..Default::default()
            };
            let _ = update_info_json(std::path::Path::new(&path), &update);
        }
    }

    Ok(())
}

fn unique_mod_ids(mod_ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    mod_ids
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

async fn get_collection(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<CollectionDetails, String> {
    collection_repo::get_collection_details(pool, id, game_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Collection not found".to_string())
}

pub async fn get_collection_preview(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<Vec<CollectionPreviewMod>, String> {
    let mut mods = collection_repo::get_collection_preview_mods(pool, id, game_id)
        .await
        .map_err(|e| e.to_string())?;

    // Also include nested collection items
    let nested_paths = collection_repo::get_nested_collection_items(pool, id)
        .await
        .map_err(|e| e.to_string())?;

    for nested_path in nested_paths {
        let path = std::path::Path::new(&nested_path);
        let display_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| nested_path.clone());

        let clean_name = display_name
            .strip_prefix("DISABLED ")
            .unwrap_or(&display_name)
            .to_string();

        let mut object_id = None;
        let mut object_name_opt = None;
        let mut object_type = None;
        let mut nested_is_safe = true; // default safe if no object match

        // Try to determine object name from the first segment of the path relative to mods folder
        // For example, if path is "Character\Barbara\BarbaraGyaruALL", the object folder is "Character" or "Barbara" depending on depth
        // A simple heuristic is to take the parent of the nested mod
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name() {
                let s = parent_name.to_string_lossy().to_string();
                let parent_clean = s.strip_prefix("DISABLED ").unwrap_or(&s).to_string();

                // Query database to see if we have an object with this folder_name
                let row: Option<(String, String, Option<String>, bool)> = sqlx::query_as(
                    "SELECT id, name, object_type, COALESCE(is_safe, 1) FROM objects WHERE game_id = ? AND (folder_path = ? COLLATE NOCASE OR name = ? COLLATE NOCASE)"
                )
                .bind(game_id)
                .bind(&parent_clean)
                .bind(&parent_clean)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((oid, oname, otype, obj_is_safe)) = row {
                    object_id = Some(oid);
                    object_name_opt = Some(oname);
                    object_type = otype;
                    nested_is_safe = obj_is_safe;
                } else {
                    object_name_opt = Some(parent_clean);
                }
            }
        }

        mods.push(CollectionPreviewMod {
            id: nested_walker::nested_mod_id(&nested_path),
            actual_name: clean_name,
            folder_path: nested_path,
            is_safe: nested_is_safe,
            object_id,
            object_name: object_name_opt,
            object_type,
        });
    }

    Ok(mods)
}

pub async fn get_active_mods_preview(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode: bool,
) -> Result<Vec<CollectionPreviewMod>, String> {
    let mut mods = collection_repo::get_active_mods_preview_mods(pool, game_id, safe_mode)
        .await
        .map_err(|e| e.to_string())?;

    log::debug!(
        "[collections] get_active_mods_preview: game_id={}, safe_mode={}, db_mods_count={}",
        game_id,
        safe_mode,
        mods.len()
    );

    // Append nested mods from filesystem walk
    let mods_path = game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;

    if let Some(ref mp) = mods_path {
        if let Ok(nested) = nested_walker::walk_nested_mods(mp) {
            let enabled_nested: Vec<_> = nested
                .into_iter()
                .filter(|n| n.is_enabled && n.is_safe == safe_mode)
                .collect();

            log::debug!(
                "[collections] nested_mods_count={} from path={}",
                enabled_nested.len(),
                mp
            );
            for nm in enabled_nested {
                mods.push(CollectionPreviewMod {
                    id: nested_walker::nested_mod_id(&nm.folder_path),
                    actual_name: nm.display_name,
                    folder_path: nm.folder_path,
                    is_safe: nm.is_safe,
                    object_id: None,
                    object_name: nm.object_name,
                    object_type: None,
                });
            }
        }
    }

    log::debug!("[collections] total_preview_mods={}", mods.len());
    Ok(mods)
}
