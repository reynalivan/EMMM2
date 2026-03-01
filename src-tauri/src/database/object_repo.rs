use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
pub struct ObjectFilter {
    pub game_id: String,
    pub search_query: Option<String>,
    pub object_type: Option<String>,
    pub safe_mode: bool,
    pub meta_filters: Option<HashMap<String, Vec<String>>>,
    pub sort_by: Option<String>,
    pub status_filter: Option<String>,
}

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub metadata: String,
    pub tags: String,
    pub is_safe: bool,
    pub is_pinned: bool,
    pub is_auto_sync: bool,
    pub thumbnail_path: Option<String>,
    pub created_at: Option<String>,
    pub mod_count: i64,
    pub enabled_count: i64,
    #[sqlx(skip)]
    pub has_naming_conflict: bool,
}

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct CategoryCount {
    pub object_type: String,
    pub count: i64,
}

pub async fn get_filtered_objects(
    pool: &SqlitePool,
    filter: &ObjectFilter,
) -> Result<Vec<ObjectSummary>, sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT
            o.id,
            o.name,
            o.folder_path,
            o.object_type,
            o.sub_category,
            o.metadata,
            o.tags,
            o.is_safe,
            o.is_pinned,
            o.is_auto_sync,
            o.thumbnail_path,
            o.created_at,
            COUNT(m.id) as mod_count,
            COUNT(CASE WHEN m.status = 'ENABLED' THEN 1 END) as enabled_count
        FROM objects o
        LEFT JOIN mods m ON m.object_id = o.id
        WHERE o.game_id = "#,
    );
    qb.push_bind(&filter.game_id);

    if filter.safe_mode {
        qb.push(" AND o.is_safe = 1");
    }

    if let Some(obj_type) = &filter.object_type {
        qb.push(" AND o.object_type = ");
        qb.push_bind(obj_type);
    }

    if let Some(sq) = &filter.search_query {
        let trimmed = sq.trim();
        if !trimmed.is_empty() {
            let search_term = format!("%{}%", trimmed.to_lowercase());
            qb.push(" AND (LOWER(o.name) LIKE ");
            qb.push_bind(search_term.clone());
            qb.push(" OR LOWER(o.tags) LIKE ");
            qb.push_bind(search_term);
            qb.push(")");
        }
    }

    if let Some(meta_filters) = &filter.meta_filters {
        for (key, values) in meta_filters {
            if !values.is_empty() {
                let safe_key = key.replace(['\'', '"'], "");
                qb.push(format!(
                    " AND JSON_EXTRACT(o.metadata, '$.{}') IN (",
                    safe_key
                ));
                let mut separated = qb.separated(", ");
                for v in values {
                    separated.push_bind(v);
                }
                separated.push_unseparated(")");
            }
        }
    }

    qb.push(" GROUP BY o.id");

    if let Some(status) = filter.status_filter.as_deref() {
        if status == "enabled" {
            qb.push(" HAVING enabled_count > 0");
        } else if status == "disabled" {
            qb.push(" HAVING mod_count > 0 AND enabled_count = 0");
        }
    }

    match filter.sort_by.as_deref() {
        Some("date") => qb.push(" ORDER BY o.is_pinned DESC, o.created_at DESC"),
        Some("rarity") => qb.push(" ORDER BY o.is_pinned DESC, CAST(JSON_EXTRACT(o.metadata, '$.rarity') AS INTEGER) DESC, o.name ASC"),
        _ => qb.push(" ORDER BY o.is_pinned DESC, o.object_type, o.name ASC"),
    };

    qb.build_query_as::<ObjectSummary>().fetch_all(pool).await
}

pub async fn get_category_counts(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode: bool,
) -> Result<Vec<CategoryCount>, sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT object_type, COUNT(*) as count FROM objects WHERE game_id = ");
    qb.push_bind(game_id);

    if safe_mode {
        qb.push(" AND is_safe = 1");
    }

    qb.push(" GROUP BY object_type ORDER BY object_type");

    qb.build_query_as::<CategoryCount>().fetch_all(pool).await
}

#[derive(Clone, Deserialize)]
pub struct CreateObjectInput {
    pub game_id: String,
    pub name: String,
    pub folder_path: Option<String>,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub is_safe: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_object(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    folder_path: &str,
    object_type: &str,
    sub_category: Option<&String>,
    is_safe: bool,
    metadata_str: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO objects (id, game_id, name, folder_path, object_type, sub_category, is_safe, is_auto_sync, tags, metadata, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 0, '[]', ?, datetime('now'))
        "#,
        id,
        game_id,
        name,
        folder_path,
        object_type,
        sub_category,
        is_safe,
        metadata_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_object(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM objects WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_mod_count_for_object(pool: &SqlitePool, id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn get_objects_folder_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query!("SELECT folder_path FROM objects WHERE game_id = ?", game_id)
        .fetch_all(pool)
        .await?;

    // Some folder_paths might be null if objects row is malformed or derived, but
    // DB schema likely has folder_path as TEXT nullable.
    // In original code, it iterated and checked `o.folder_path`.
    let mut paths = Vec::new();
    for row in rows {
        if let Some(fp) = row.folder_path {
            paths.push(fp);
        }
    }
    Ok(paths)
}

pub async fn update_object_folder_path(
    pool: &SqlitePool,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET folder_path = ? WHERE game_id = ? AND folder_path = ?")
        .bind(new_path)
        .bind(game_id)
        .bind(old_path)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct UpdateObjectInput {
    pub name: Option<String>,
    pub object_type: Option<String>,
    pub sub_category: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub thumbnail_path: Option<String>,
    pub is_safe: Option<bool>,
    pub is_auto_sync: Option<bool>,
    pub tags: Option<Vec<String>>,
}

pub async fn update_object(
    pool: &SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE objects SET ");
    let mut is_first = true;

    if let Some(name) = &updates.name {
        if !is_first {
            qb.push(", ");
        }
        qb.push("name = ");
        qb.push_bind(name.trim().to_string());
        is_first = false;
    }
    if let Some(obj_type) = &updates.object_type {
        if !is_first {
            qb.push(", ");
        }
        qb.push("object_type = ");
        qb.push_bind(obj_type);
        is_first = false;
    }
    if let Some(sub) = &updates.sub_category {
        if !is_first {
            qb.push(", ");
        }
        qb.push("sub_category = ");
        qb.push_bind(sub);
        is_first = false;
    }
    if let Some(meta) = &updates.metadata {
        if !is_first {
            qb.push(", ");
        }
        qb.push("metadata = ");
        qb.push_bind(meta.to_string());
        is_first = false;
    }
    if let Some(thumb) = &updates.thumbnail_path {
        if !is_first {
            qb.push(", ");
        }
        qb.push("thumbnail_path = ");
        qb.push_bind(thumb);
        is_first = false;
    }
    if let Some(safe) = updates.is_safe {
        if !is_first {
            qb.push(", ");
        }
        qb.push("is_safe = ");
        qb.push_bind(safe);
        is_first = false;
    }
    if let Some(auto) = updates.is_auto_sync {
        if !is_first {
            qb.push(", ");
        }
        qb.push("is_auto_sync = ");
        qb.push_bind(auto);
        is_first = false;
    }
    if let Some(tags) = &updates.tags {
        if !is_first {
            qb.push(", ");
        }
        qb.push("tags = ");
        qb.push_bind(serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string()));
        is_first = false;
    }

    if is_first {
        return Ok(());
    }

    qb.push(" WHERE id = ");
    qb.push_bind(id);

    qb.build().execute(pool).await?;
    Ok(())
}

pub async fn set_is_safe(pool: &SqlitePool, id: &str, safe: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET is_safe = ? WHERE id = ?")
        .bind(safe)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_characters_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    use sqlx::Row;
    let rows =
        sqlx::query("SELECT id, name FROM objects WHERE game_id = ? AND object_type = 'Character'")
            .bind(game_id)
            .fetch_all(pool)
            .await?;

    let mut result = Vec::new();
    for row in rows {
        result.push((row.try_get("id")?, row.try_get("name")?));
    }
    Ok(result)
}

pub async fn get_folder_path(pool: &SqlitePool, id: &str) -> Result<Option<String>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query("SELECT folder_path FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        Ok(r.try_get("folder_path").ok())
    } else {
        Ok(None)
    }
}

pub async fn get_game_object_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, sqlx::Error> {
    sqlx::query_as::<_, crate::services::scanner::core::types::GameObject>(
        "SELECT * FROM objects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn set_is_pinned(
    pool: &SqlitePool,
    id: &str,
    is_pinned: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET is_pinned = ? WHERE id = ?")
        .bind(is_pinned)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn ensure_object_exists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    game_id: &str,
    folder_path: &str,
    obj_name: &str,
    obj_type: &str,
    db_thumbnail: Option<&str>,
    db_tags_json: &str,
    db_metadata_json: &str,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    use sqlx::Row;
    let existing = sqlx::query(
        "SELECT id, name, folder_path, object_type, thumbnail_path, tags, metadata FROM objects WHERE game_id = ? AND (folder_path = ? COLLATE NOCASE OR name = ? COLLATE NOCASE)",
    )
    .bind(game_id)
    .bind(folder_path)
    .bind(obj_name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(row) = existing {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let existing_name: String = row.try_get("name").map_err(|e| e.to_string())?;
        let existing_fp: String = row.try_get("folder_path").unwrap_or_default();
        let existing_type: String = row
            .try_get("object_type")
            .unwrap_or_else(|_| "Other".to_string());

        if existing_fp != folder_path {
            sqlx::query("UPDATE objects SET folder_path = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        if (existing_name != obj_name || existing_type != obj_type) && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET name = ?, object_type = ? WHERE id = ?")
                .bind(obj_name)
                .bind(obj_type)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());
        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        return Ok(id);
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, thumbnail_path, tags, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
        .bind(&new_id)
        .bind(game_id)
        .bind(obj_name)
        .bind(folder_path)
        .bind(obj_type)
        .bind(db_thumbnail)
        .bind(db_tags_json)
        .bind(db_metadata_json)
        .execute(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;

    *new_objects_count += 1;
    Ok(new_id)
}

pub async fn update_object_is_safe_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    is_safe: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET is_safe = ? WHERE id = ?")
        .bind(is_safe)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_ghost_objects_gc(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM objects WHERE game_id = $1 AND NOT EXISTS (SELECT 1 FROM mods WHERE object_id = objects.id)"
    )
    .bind(game_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get_object_name_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT name FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
#[path = "tests/object_repo_test.rs"]
mod tests;
