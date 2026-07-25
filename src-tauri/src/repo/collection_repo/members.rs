//! Reads of the split member tables (`collection_mods` / `collection_objects` /
//! `collection_roots`), both pool-based and inside an open transaction.

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::mapping::parse_warnings_json;
use crate::domain::collection::{CollectionMod, CollectionObject, CollectionRoot};
use crate::domain::errors::CollectionError;

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

    Ok(rows
        .iter()
        .map(|r| CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: r.get("collection_id"),
            mod_id: r.get("mod_id"),
            mod_path: r.get("mod_path"),
            mod_path_key: r.get("mod_path_key"),
            object_id: r.get("object_id"),
            display_name: r.get("display_name"),
            preview_path: r.get("preview_path"),
            node_type: r.get("node_type"),
            warnings: parse_warnings_json(r.try_get("warnings_json").ok()),
            is_enabled: true,
        })
        .collect())
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

    Ok(rows
        .iter()
        .map(|r| CollectionObject {
            kind: crate::domain::collection::MemberKind::Object,
            collection_id: r.get("collection_id"),
            object_id: r.get("object_id"),
            is_enabled: r.get::<i32, _>("is_enabled") != 0,
            display_name: r.get("display_name"),
            path_key: r.get("path_key"),
        })
        .collect())
}

pub async fn get_roots(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionRoot>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT collection_id, root_path, root_path_key, display_name, display_name_key,
                  object_id, object_name, object_type, root_kind, is_safe, is_enabled,
                  thumbnail_hint, corridor_source
        FROM collection_roots WHERE collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| CollectionRoot {
            kind: crate::domain::collection::MemberKind::Root,
            collection_id: r.get("collection_id"),
            root_path: r.get("root_path"),
            root_path_key: r.get("root_path_key"),
            display_name: r.get("display_name"),
            display_name_key: r.get("display_name_key"),
            object_id: r.get("object_id"),
            object_name: r.get("object_name"),
            object_type: r.get("object_type"),
            root_kind: r.get("root_kind"),
            is_safe: r.get::<i32, _>("is_safe") != 0,
            is_enabled: r.get::<i32, _>("is_enabled") != 0,
            thumbnail_hint: r.get("thumbnail_hint"),
            corridor_source: r.get("corridor_source"),
        })
        .collect())
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

    Ok(rows
        .iter()
        .map(|r| CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: r.try_get("collection_id").unwrap_or_default(),
            mod_id: r.try_get("mod_id").unwrap_or_default(),
            mod_path: r.try_get("mod_path").unwrap_or_default(),
            mod_path_key: r.try_get("mod_path_key").unwrap_or_default(),
            object_id: r.try_get("object_id").unwrap_or_default(),
            display_name: None,
            preview_path: r.try_get("preview_path").unwrap_or_default(),
            node_type: r.try_get("node_type").unwrap_or_default(),
            warnings: parse_warnings_json(r.try_get("warnings_json").ok()),
            is_enabled: true,
        })
        .collect())
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

    Ok(rows
        .iter()
        .map(|r| CollectionObject {
            kind: crate::domain::collection::MemberKind::Object,
            collection_id: r.try_get("collection_id").unwrap_or_default(),
            object_id: r.try_get("object_id").unwrap_or_default(),
            is_enabled: r.try_get::<i32, _>("is_enabled").unwrap_or(1) != 0,
            display_name: None,
            path_key: None,
        })
        .collect())
}
