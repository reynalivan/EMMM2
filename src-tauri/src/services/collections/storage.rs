use super::types::{
    Collection, CollectionDetails, CollectionPreviewMod, CreateCollectionInput,
    UpdateCollectionInput,
};
use crate::database::collection_repo;
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
        mod_ids = collection_repo::get_enabled_mod_ids(&mut tx, &input.game_id)
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

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(CollectionDetails {
        collection: Collection {
            id,
            name: input.name.trim().to_string(),
            game_id: input.game_id,
            is_safe_context: input.is_safe_context,
            member_count: mod_ids.len(),
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
        collection_repo::update_collection_name(&mut tx, &input.id, &input.game_id, name_trimmed)
            .await
            .map_err(|e| e.to_string())?;

        // Rename preset_name in all current members' info.json if name changed
        if new_name != old_name {
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

    collection_repo::delete_collection(pool, id, game_id)
        .await
        .map_err(|e| e.to_string())?;

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
    collection_repo::get_collection_preview_mods(pool, id, game_id)
        .await
        .map_err(|e| e.to_string())
}
