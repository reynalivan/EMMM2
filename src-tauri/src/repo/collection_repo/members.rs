//! Reads of the split member tables (`collection_mods` / `collection_objects` /
//! `collection_roots`), both pool-based and inside an open transaction.

use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection, SqlitePool};

use super::mapping::parse_warnings_json;
use crate::domain::collection::{CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;

/// Row mapper shared by the pool and transaction reads. The transaction queries
/// skip the display joins, so every column is read tolerantly: a missing column
/// yields the same value it would if the join produced NULL.
fn map_mod_row(row: &SqliteRow) -> CollectionMod {
    CollectionMod {
        kind: crate::domain::collection::MemberKind::Mod,
        collection_id: row.try_get("collection_id").unwrap_or_default(),
        mod_id: row.try_get("mod_id").unwrap_or_default(),
        mod_path: row.try_get("mod_path").unwrap_or_default(),
        mod_path_key: row.try_get("mod_path_key").unwrap_or_default(),
        object_id: row.try_get("object_id").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or_default(),
        preview_path: row.try_get("preview_path").unwrap_or_default(),
        node_type: row.try_get("node_type").unwrap_or_default(),
        warnings: parse_warnings_json(row.try_get("warnings_json").ok()),
        is_enabled: true,
    }
}

fn map_object_row(row: &SqliteRow) -> CollectionObject {
    CollectionObject {
        kind: crate::domain::collection::MemberKind::Object,
        collection_id: row.try_get("collection_id").unwrap_or_default(),
        object_id: row.try_get("object_id").unwrap_or_default(),
        is_enabled: row.try_get::<i32, _>("is_enabled").unwrap_or(1) != 0,
        display_name: row.try_get("display_name").unwrap_or_default(),
        path_key: row.try_get("path_key").unwrap_or_default(),
    }
}

pub async fn get_mods(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionMod>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT cm.collection_id, cm.mod_id, cm.mod_path, cm.mod_path_key, cm.object_id,
                  cm.preview_path, cm.node_type, cm.warnings_json,
                  m.actual_name as display_name
           FROM collection_mods cm
           LEFT JOIN mods m ON cm.mod_id = m.id
           WHERE cm.collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_mod_row).collect())
}

pub async fn get_objects(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT co.collection_id, co.object_id, co.is_enabled, 
                  o.name as display_name, o.folder_path as path_key
           FROM collection_objects co
           LEFT JOIN objects o ON co.object_id = o.id
           WHERE co.collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_object_row).collect())
}

/// Member mods inside an open transaction (no mods join; display_name stays None).
pub async fn get_mods_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<Vec<CollectionMod>, CollectionError> {
    let rows = sqlx::query(
        "SELECT collection_id, mod_id, mod_path, mod_path_key, object_id, preview_path, node_type, warnings_json FROM collection_mods WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(rows.iter().map(map_mod_row).collect())
}

/// Member objects inside an open transaction (no objects join).
pub async fn get_objects_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
        "SELECT collection_id, object_id, is_enabled FROM collection_objects WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(rows.iter().map(map_object_row).collect())
}
