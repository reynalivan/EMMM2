use super::types::{
    Collection, CollectionDetails, CollectionPreviewMod, CreateCollectionInput,
    UpdateCollectionInput,
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
        r#"
        SELECT c.id, c.name, c.game_id, c.is_safe_context, COUNT(ci.mod_id) as member_count, COALESCE(c.is_last_unsaved, 0) as is_last_unsaved
        FROM collections c
        LEFT JOIN collection_items ci ON c.id = ci.collection_id
        WHERE c.game_id = ? AND c.is_safe_context = 1
        GROUP BY c.id
        ORDER BY c.is_last_unsaved DESC, c.name
        "#
    } else {
        r#"
        SELECT c.id, c.name, c.game_id, c.is_safe_context, COUNT(ci.mod_id) as member_count, COALESCE(c.is_last_unsaved, 0) as is_last_unsaved
        FROM collections c
        LEFT JOIN collection_items ci ON c.id = ci.collection_id
        WHERE c.game_id = ?
        GROUP BY c.id
        ORDER BY c.is_last_unsaved DESC, c.name
        "#
    };

    sqlx::query_as::<_, (String, String, String, bool, i64, bool)>(sql)
        .bind(game_id)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, name, gid, safe, count, is_last_unsaved)| Collection {
                    id,
                    name,
                    game_id: gid,
                    is_safe_context: safe,
                    member_count: count as usize,
                    is_last_unsaved,
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

    let name_trimmed = input.name.trim();

    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM collections WHERE game_id = ? AND name = ? AND is_safe_context = ?",
    )
    .bind(&input.game_id)
    .bind(name_trimmed)
    .bind(input.is_safe_context)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Err(format!(
            "A collection named '{}' already exists.",
            name_trimmed
        ));
    }

    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(name_trimmed)
        .bind(&input.game_id)
        .bind(input.is_safe_context)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut mod_ids = input.mod_ids;
    if input.auto_snapshot.unwrap_or(false) {
        mod_ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM mods WHERE game_id = ? AND status = 'ENABLED'",
        )
        .bind(&input.game_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    let mod_ids = unique_mod_ids(mod_ids);

    // Fetch folder_paths for all selected mods
    let mut paths: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if !mod_ids.is_empty() {
        let mut qb: sqlx::QueryBuilder<'_, sqlx::Sqlite> =
            sqlx::QueryBuilder::new("SELECT id, folder_path FROM mods WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in &mod_ids {
            sep.push_bind(id);
        }
        qb.push(")");
        let rows: Vec<(String, String)> = qb
            .build_query_as()
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        for (id, path) in rows {
            paths.insert(id, path);
        }
    }

    for mod_id in &mod_ids {
        let mod_path = paths.get(mod_id).map(|p| p.as_str());
        sqlx::query("INSERT OR IGNORE INTO collection_items (collection_id, mod_id, mod_path) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(mod_id)
            .bind(mod_path)
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
            sqlx::query(
                "INSERT OR IGNORE INTO collection_items (collection_id, mod_id) VALUES (?, ?)",
            )
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
    let (cid, name, gid, safe, is_last_unsaved): (String, String, String, bool, bool) = sqlx::query_as(
        "SELECT id, name, game_id, is_safe_context, COALESCE(is_last_unsaved, 0) FROM collections WHERE id = ? AND game_id = ?",
    )
    .bind(id)
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("Collection not found")?;

    let mod_ids = sqlx::query_scalar(
        "SELECT mod_id FROM collection_items WHERE collection_id = ? ORDER BY mod_id",
    )
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
            member_count: mod_ids.len(),
            is_last_unsaved,
        },
        mod_ids,
    })
}

pub async fn get_collection_preview(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<Vec<CollectionPreviewMod>, String> {
    let sql = r#"
        SELECT
            m.id,
            m.actual_name,
            m.folder_path,
            COALESCE(m.is_safe, 0) as is_safe,
            m.object_id,
            o.name as object_name,
            o.object_type
        FROM collection_items ci
        JOIN mods m ON ci.mod_id = m.id
        LEFT JOIN objects o ON m.object_id = o.id
        WHERE ci.collection_id = ? AND m.game_id = ?
        ORDER BY o.name ASC, m.actual_name ASC
    "#;

    sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            bool,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(sql)
    .bind(id)
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(
                |(id, actual_name, folder_path, is_safe, object_id, object_name, object_type)| {
                    CollectionPreviewMod {
                        id,
                        actual_name,
                        folder_path,
                        is_safe,
                        object_id,
                        object_name,
                        object_type,
                    }
                },
            )
            .collect()
    })
    .map_err(|e| e.to_string())
}
