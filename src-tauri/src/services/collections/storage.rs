use super::types::{
    Collection, CollectionDetails, CreateCollectionInput, ExportCollectionItem,
    ExportCollectionPayload, ImportCollectionResult, UpdateCollectionInput,
};
use sqlx::SqlitePool;
use std::collections::HashSet;
use uuid::Uuid;

pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<Vec<Collection>, String> {
    let sql = if safe_mode_enabled {
        "SELECT id, name, game_id, is_safe_context FROM collections WHERE game_id = ? AND is_safe_context = 1 ORDER BY name"
    } else {
        "SELECT id, name, game_id, is_safe_context FROM collections WHERE game_id = ? ORDER BY name"
    };

    sqlx::query_as::<_, (String, String, String, bool)>(sql)
        .bind(game_id)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, name, gid, safe)| Collection {
                    id,
                    name,
                    game_id: gid,
                    is_safe_context: safe,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub async fn create_collection(
    pool: &SqlitePool,
    input: CreateCollectionInput,
) -> Result<CollectionDetails, String> {
    let id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(input.name.trim())
        .bind(&input.game_id)
        .bind(input.is_safe_context)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let mod_ids = unique_mod_ids(input.mod_ids);
    for mod_id in &mod_ids {
        sqlx::query("INSERT OR IGNORE INTO collection_items (collection_id, mod_id) VALUES (?, ?)")
            .bind(&id)
            .bind(mod_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(CollectionDetails {
        collection: Collection {
            id,
            name: input.name.trim().to_string(),
            game_id: input.game_id,
            is_safe_context: input.is_safe_context,
        },
        mod_ids,
    })
}

pub async fn update_collection(
    pool: &SqlitePool,
    input: UpdateCollectionInput,
) -> Result<CollectionDetails, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    if let Some(name) = input.name.as_ref() {
        sqlx::query("UPDATE collections SET name = ? WHERE id = ? AND game_id = ?")
            .bind(name.trim())
            .bind(&input.id)
            .bind(&input.game_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(safe) = input.is_safe_context {
        sqlx::query("UPDATE collections SET is_safe_context = ? WHERE id = ? AND game_id = ?")
            .bind(safe)
            .bind(&input.id)
            .bind(&input.game_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(mod_ids) = input.mod_ids.as_ref() {
        let unique = unique_mod_ids(mod_ids.clone());
        sqlx::query("DELETE FROM collection_items WHERE collection_id = ?")
            .bind(&input.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        for mod_id in &unique {
            sqlx::query("INSERT OR IGNORE INTO collection_items (collection_id, mod_id) VALUES (?, ?)")
                .bind(&input.id)
                .bind(mod_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    get_collection(pool, &input.id, &input.game_id).await
}

pub async fn delete_collection(pool: &SqlitePool, id: &str, game_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM collections WHERE id = ? AND game_id = ?")
        .bind(id)
        .bind(game_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn export_collection(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<ExportCollectionPayload, String> {
    let (name, is_safe_context): (String, bool) =
        sqlx::query_as("SELECT name, is_safe_context FROM collections WHERE id = ? AND game_id = ?")
            .bind(collection_id)
            .bind(game_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Collection not found")?;

    let items = sqlx::query_as::<_, (String, String, String)>(
        "SELECT m.id, m.actual_name, m.folder_path FROM collection_items ci JOIN mods m ON m.id = ci.mod_id WHERE ci.collection_id = ? ORDER BY m.actual_name",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?
    .into_iter()
    .map(|(mod_id, actual_name, folder_path)| ExportCollectionItem {
        mod_id,
        actual_name,
        folder_path,
    })
    .collect();

    Ok(ExportCollectionPayload {
        version: 1,
        name,
        game_id: game_id.to_string(),
        is_safe_context,
        items,
    })
}

pub async fn import_collection(
    pool: &SqlitePool,
    game_id: &str,
    payload: ExportCollectionPayload,
) -> Result<ImportCollectionResult, String> {
    let mut matched = Vec::new();
    let mut missing = Vec::new();

    for item in payload.items {
        let resolved: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mods WHERE game_id = ? AND id = ? UNION SELECT id FROM mods WHERE game_id = ? AND actual_name = ? UNION SELECT id FROM mods WHERE game_id = ? AND folder_path = ? LIMIT 1",
        )
        .bind(game_id)
        .bind(&item.mod_id)
        .bind(game_id)
        .bind(&item.actual_name)
        .bind(game_id)
        .bind(&item.folder_path)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        match resolved {
            Some(mod_id) => matched.push(mod_id),
            None => missing.push(item.actual_name),
        }
    }

    let created = create_collection(
        pool,
        CreateCollectionInput {
            name: payload.name,
            game_id: game_id.to_string(),
            is_safe_context: payload.is_safe_context,
            mod_ids: matched.clone(),
        },
    )
    .await?;

    Ok(ImportCollectionResult {
        collection_id: created.collection.id,
        imported_count: matched.len(),
        missing,
    })
}

fn unique_mod_ids(mod_ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    mod_ids
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

async fn get_collection(pool: &SqlitePool, id: &str, game_id: &str) -> Result<CollectionDetails, String> {
    let (cid, name, gid, safe): (String, String, String, bool) = sqlx::query_as(
        "SELECT id, name, game_id, is_safe_context FROM collections WHERE id = ? AND game_id = ?",
    )
    .bind(id)
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("Collection not found")?;

    let mod_ids = sqlx::query_scalar("SELECT mod_id FROM collection_items WHERE collection_id = ? ORDER BY mod_id")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CollectionDetails {
        collection: Collection {
            id: cid,
            name,
            game_id: gid,
            is_safe_context: safe,
        },
        mod_ids,
    })
}
