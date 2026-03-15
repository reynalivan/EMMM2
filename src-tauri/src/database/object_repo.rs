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

#[derive(Clone, Serialize)]
pub struct GetObjectsResult {
    pub objects: Vec<ObjectSummary>,
    pub lost_objects: Vec<String>,
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
    pub is_pinned: bool,
    pub is_auto_sync: bool,
    pub thumbnail_path: Option<String>,
    pub created_at: Option<String>,
    pub mod_count: i64,
    pub enabled_count: i64,
    pub is_object_disabled: bool,
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
    // Phase 14: Mutually Exclusive Corridors (ObjectList Visibility)
    // ObjectList ALWAYS shows all objects.
    // Safe mode = count only Safe mods (is_safe = 1)
    // Unsafe mode = count only Unsafe mods (is_safe = 0)
    let count_expr = if filter.safe_mode {
        r#"
            COUNT(CASE WHEN m.is_safe = 1 THEN m.id END) as mod_count,
            COUNT(CASE WHEN m.is_safe = 1 AND m.status = 'ENABLED' THEN 1 END) as enabled_count,
        "#
    } else {
        r#"
            COUNT(CASE WHEN m.is_safe = 0 THEN m.id END) as mod_count,
            COUNT(CASE WHEN m.is_safe = 0 AND m.status = 'ENABLED' THEN 1 END) as enabled_count,
        "#
    };

    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
        r#"
        SELECT
            o.id,
            o.name,
            o.folder_path,
            o.object_type,
            o.sub_category,
            o.metadata,
            o.tags,
            o.is_pinned,
            o.is_auto_sync,
            o.thumbnail_path,
            o.created_at,
            {}
            (o.folder_path LIKE 'DISABLED %' OR o.folder_path LIKE '%/DISABLED %' OR o.folder_path LIKE '%\\DISABLED %') as is_object_disabled
        FROM objects o
        LEFT JOIN mods m ON m.object_id = o.id
        WHERE o.game_id = "#,
        count_expr
    ));
    qb.push_bind(&filter.game_id);

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
    _safe_mode: bool,
) -> Result<Vec<CategoryCount>, sqlx::Error> {
    // Phase 1 fix: always count ALL objects regardless of safe mode.
    // Category badges should show total counts; individual object counts
    // are zeroed for unsafe objects at the object level.
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT object_type, COUNT(*) as count FROM objects WHERE game_id = ");
    qb.push_bind(game_id);

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
    pub metadata: Option<serde_json::Value>,
    pub thumbnail_url: Option<String>,
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
    metadata_str: &str,
    thumbnail_path: Option<&String>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO objects (id, game_id, name, folder_path, object_type, sub_category, is_auto_sync, tags, metadata, thumbnail_path, created_at)
        VALUES (?, ?, ?, ?, ?, ?, 0, '[]', ?, ?, datetime('now'))
        "#,
        id,
        game_id,
        name,
        folder_path,
        object_type,
        sub_category,
        metadata_str,
        thumbnail_path
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

/// Atomically delete an object folder and all its child mods from the DB.
///
/// Used when the watcher detects a depth=1 `Removed` event (an entire object
/// folder was deleted from disk). The operation runs inside a single transaction:
/// 1. Delete all `mods` rows whose `folder_path` starts with `{folder_path}/` or `{folder_path}\`
/// 2. Delete the `objects` row with `folder_path = folder_path AND game_id = game_id`
///
/// Idempotent — safe to call even if the object does not exist.
pub async fn delete_object_and_mods_by_folder(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
) -> Result<u64, sqlx::Error> {
    // Delete child mods (both slash styles to be OS-agnostic)
    let prefix_fwd = format!("{}/", folder_path);
    let prefix_back = format!("{}\\", folder_path);
    let mods_deleted = sqlx::query(
        "DELETE FROM mods WHERE game_id = ? AND (folder_path LIKE ? OR folder_path LIKE ?)",
    )
    .bind(game_id)
    .bind(format!("{}%", prefix_fwd))
    .bind(format!("{}%", prefix_back))
    .execute(&mut *conn)
    .await?
    .rows_affected();

    // Delete the object itself
    sqlx::query("DELETE FROM objects WHERE game_id = ? AND folder_path = ?")
        .bind(game_id)
        .bind(folder_path)
        .execute(&mut *conn)
        .await?;

    log::info!(
        "delete_object_and_mods_by_folder: removed object folder='{}' game='{}', {} child mods deleted",
        folder_path, game_id, mods_deleted
    );
    Ok(mods_deleted)
}

pub async fn get_mod_count_for_object(pool: &SqlitePool, id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

/// Delete all mod rows belonging to an object (cascade helper).
pub async fn delete_mods_for_object(
    pool: &SqlitePool,
    object_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM mods WHERE object_id = ?")
        .bind(object_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
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

pub async fn update_object_folder_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    // If the object's name is identical to its old folder path, rename the object as well
    // so that local disk renames are reflected in the UI for objects that haven't been custom-named.
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             name = CASE WHEN name = ? OR name = ? THEN ? ELSE name END
         WHERE game_id = ? AND folder_path = ?",
    )
    .bind(new_path)
    .bind(old_path)
    .bind(old_path.to_ascii_lowercase()) // Simple case fallback, though exact match covers most
    .bind(new_path)
    .bind(game_id)
    .bind(old_path)
    .execute(executor)
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
    conn: &mut sqlx::SqliteConnection,
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
    let existing_rows = sqlx::query(
        "SELECT id, name, folder_path, object_type, thumbnail_path, tags, metadata FROM objects WHERE game_id = ? AND (folder_path = ? COLLATE NOCASE OR name = ? COLLATE NOCASE)",
    )
    .bind(game_id)
    .bind(folder_path)
    .bind(obj_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    let mut match_name = None;
    let mut match_folder = None;

    for row in existing_rows {
        let id: String = row.try_get("id").unwrap_or_default();
        let name: String = row.try_get("name").unwrap_or_default();
        let fp: String = row.try_get("folder_path").unwrap_or_default();
        let o_type: String = row
            .try_get("object_type")
            .unwrap_or_else(|_| "Other".to_string());
        let thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        let tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());

        if name.eq_ignore_ascii_case(obj_name) {
            match_name = Some((
                id.clone(),
                name.clone(),
                fp.clone(),
                o_type.clone(),
                thumb.clone(),
                tags.clone(),
                meta.clone(),
            ));
        }
        if fp.eq_ignore_ascii_case(folder_path) {
            match_folder = Some((id, name, fp, o_type, thumb, tags, meta));
        }
    }

    if let Some((
        id,
        _existing_name,
        existing_fp,
        existing_type,
        existing_thumb,
        existing_tags,
        existing_meta,
    )) = match_name
    {
        let has_folder_conflict = match_folder.as_ref().is_some_and(|f| f.0 != id);

        if existing_fp != folder_path && !has_folder_conflict {
            sqlx::query("UPDATE objects SET folder_path = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if _existing_name != obj_name {
            sqlx::query("UPDATE objects SET name = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_type != obj_type && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET object_type = ? WHERE id = ?")
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        return Ok(id);
    } else if let Some((
        id,
        _existing_name,
        existing_fp,
        _existing_type,
        existing_thumb,
        existing_tags,
        existing_meta,
    )) = match_folder
    {
        if existing_fp != folder_path {
            sqlx::query("UPDATE objects SET folder_path = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET name = ?, object_type = ? WHERE id = ?")
                .bind(obj_name)
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            sqlx::query("UPDATE objects SET name = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut *conn)
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
    .execute(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    *new_objects_count += 1;
    Ok(new_id)
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
