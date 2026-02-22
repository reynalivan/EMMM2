use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use crate::types::errors::CommandResult;

#[derive(Deserialize)]
pub struct ObjectFilter {
    pub game_id: String,
    pub search_query: Option<String>,
    pub object_type: Option<String>,
    pub safe_mode: bool,
    pub meta_filters: Option<HashMap<String, Vec<String>>>,
    pub sort_by: Option<String>,
    pub status_filter: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
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
}

#[tauri::command]
pub async fn get_objects_cmd(
    filter: ObjectFilter,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<Vec<ObjectSummary>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT
            o.id,
            o.name,
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
                // Ensure key is safe from SQL injection by replacing simple bad chars
                // (Since we dictate the schema, key is usually alphabetic. But just to be sure)
                let safe_key = key.replace('\'', "").replace('"', "");
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

    let objects = qb
        .build_query_as::<ObjectSummary>()
        .fetch_all(&*pool)
        .await?;

    Ok(objects)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CategoryCount {
    pub object_type: String,
    pub count: i64,
}

#[tauri::command]
pub async fn get_category_counts_cmd(
    game_id: String,
    safe_mode: bool,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<Vec<CategoryCount>> {
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT object_type, COUNT(*) as count FROM objects WHERE game_id = ");
    qb.push_bind(game_id);

    if safe_mode {
        qb.push(" AND is_safe = 1");
    }

    qb.push(" GROUP BY object_type ORDER BY object_type");

    let counts = qb
        .build_query_as::<CategoryCount>()
        .fetch_all(&*pool)
        .await?;

    Ok(counts)
}

#[derive(Deserialize)]
pub struct CreateObjectInput {
    pub game_id: String,
    pub name: String,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub is_safe: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[tauri::command]
pub async fn create_object_cmd(
    input: CreateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<String> {
    let id = Uuid::new_v4().to_string();
    let is_safe = input.is_safe.unwrap_or(true);
    let metadata_str = input
        .metadata
        .map(|m| m.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let res = sqlx::query!(
        r#"
        INSERT INTO objects (id, game_id, name, object_type, sub_category, is_safe, is_auto_sync, tags, metadata, created_at)
        VALUES (?, ?, ?, ?, ?, ?, 0, '[]', ?, datetime('now'))
        "#,
        id,
        input.game_id,
        input.name,
        input.object_type,
        input.sub_category,
        is_safe,
        metadata_str
    )
    .execute(&*pool)
    .await;

    match res {
        Ok(_) => Ok(id),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique constraint failed") || msg.contains("idx_objects_game_name") {
                Err(crate::types::errors::CommandError::Database(format!(
                    "An object named '{}' already exists for this game.",
                    input.name.trim()
                )))
            } else {
                Err(e.into())
            }
        }
    }
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

#[tauri::command]
pub async fn update_object_cmd(
    id: String,
    updates: UpdateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> CommandResult<()> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE objects SET ");
    let mut is_first = true;

    if let Some(name) = updates.name {
        if !is_first {
            qb.push(", ");
        }
        qb.push("name = ");
        qb.push_bind(name.trim().to_string());
        is_first = false;
    }
    if let Some(obj_type) = updates.object_type {
        if !is_first {
            qb.push(", ");
        }
        qb.push("object_type = ");
        qb.push_bind(obj_type);
        is_first = false;
    }
    if let Some(sub) = updates.sub_category {
        if !is_first {
            qb.push(", ");
        }
        qb.push("sub_category = ");
        qb.push_bind(sub);
        is_first = false;
    }
    if let Some(meta) = updates.metadata {
        if !is_first {
            qb.push(", ");
        }
        qb.push("metadata = ");
        qb.push_bind(meta.to_string());
        is_first = false;
    }
    if let Some(thumb) = updates.thumbnail_path {
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
    if let Some(tags) = updates.tags {
        if !is_first {
            qb.push(", ");
        }
        qb.push("tags = ");
        qb.push_bind(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()));
        is_first = false;
    }

    if is_first {
        return Ok(());
    }

    qb.push(" WHERE id = ");
    qb.push_bind(id);

    let res = qb.build().execute(&*pool).await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("unique constraint failed") || msg.contains("idx_objects_game_name") {
                Err(crate::types::errors::CommandError::Database(
                    "An object with that name already exists.".to_string(),
                ))
            } else {
                Err(e.into())
            }
        }
    }
}

#[tauri::command]
pub async fn delete_object_cmd(id: String, pool: State<'_, sqlx::SqlitePool>) -> CommandResult<()> {
    // Note: Due to foreign keys (if strict), this might fail if mods are attached.
    // In our DB schema, `object_id` on mods is nullable, or ON DELETE SET NULL/CASCADE.
    sqlx::query!("DELETE FROM objects WHERE id = ?", id)
        .execute(&*pool)
        .await?;
    Ok(())
}
