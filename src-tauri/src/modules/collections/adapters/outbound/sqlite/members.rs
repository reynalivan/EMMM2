//! Reads of canonical member tables (`collection_mods` / `collection_objects`),
//! both pool-based and inside an open transaction.

use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection, SqlitePool};

use super::mapping::parse_warnings_json;
use crate::modules::collections::domain::collection::{CollectionMod, CollectionObject};
use crate::shared::errors::CollectionError;

fn optional_string_column(
    row: &SqliteRow,
    column: &'static str,
) -> Result<Option<String>, sqlx::Error> {
    match row.try_get(column) {
        Ok(value) => Ok(value),
        Err(sqlx::Error::ColumnNotFound(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

fn map_mod_row(row: &SqliteRow) -> Result<CollectionMod, CollectionError> {
    Ok(CollectionMod {
        kind: crate::modules::collections::domain::collection::MemberKind::Mod,
        collection_id: row.try_get("collection_id")?,
        mod_id: row.try_get("mod_id")?,
        mod_path: row.try_get("mod_path")?,
        mod_path_key: row.try_get("mod_path_key")?,
        object_id: row.try_get("effective_object_id")?,
        display_name: optional_string_column(row, "display_name")?,
        preview_path: row.try_get("preview_path")?,
        node_type: row.try_get("node_type")?,
        warnings: parse_warnings_json(row.try_get("warnings_json")?)
            .map_err(|error| CollectionError::Db(format!("Invalid warnings JSON: {error}")))?,
        is_enabled: true,
        is_safe: row.try_get::<i32, _>("is_safe")? != 0,
        safety_source: optional_string_column(row, "safety_source")?,
    })
}

fn map_object_row(row: &SqliteRow) -> Result<CollectionObject, sqlx::Error> {
    Ok(CollectionObject {
        kind: crate::modules::collections::domain::collection::MemberKind::Object,
        collection_id: row.try_get("collection_id")?,
        object_id: row.try_get("effective_object_id")?,
        is_enabled: row.try_get::<i32, _>("is_enabled")? != 0,
        display_name: optional_string_column(row, "display_name")?,
        path_key: optional_string_column(row, "path_key")?,
    })
}

pub async fn get_mods(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionMod>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT cm.collection_id, cm.mod_id, cm.mod_path, cm.mod_path_key,
                  COALESCE(cm.object_id, cm.object_ref_key) AS effective_object_id,
                  cm.preview_path, cm.node_type, cm.warnings_json,
                  cm.is_safe, cm.safety_source,
                  m.actual_name as display_name
           FROM collection_mods cm
           LEFT JOIN mods m ON cm.mod_id = m.id
           WHERE cm.collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    rows.iter().map(map_mod_row).collect()
}

pub async fn get_objects(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT co.collection_id,
                  COALESCE(co.object_id, co.object_ref_key) AS effective_object_id,
                  co.is_enabled,
                  COALESCE(o.name, co.object_display_name) as display_name,
                  COALESCE(o.folder_path, co.object_ref_key) as path_key
           FROM collection_objects co
           LEFT JOIN objects o ON co.object_id = o.id
           WHERE co.collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(map_object_row)
        .collect::<Result<Vec<_>, _>>()?)
}

/// Member mods inside an open transaction (no mods join; display_name stays None).
pub async fn get_mods_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<Vec<CollectionMod>, CollectionError> {
    let rows = sqlx::query(
        "SELECT collection_id, mod_id, mod_path, mod_path_key, COALESCE(object_id, object_ref_key) AS effective_object_id, preview_path, node_type, warnings_json, is_safe, safety_source FROM collection_mods WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;

    rows.iter().map(map_mod_row).collect()
}

/// Member objects inside an open transaction (no objects join).
pub async fn get_objects_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
        "SELECT collection_id, COALESCE(object_id, object_ref_key) AS effective_object_id, is_enabled, object_display_name AS display_name, object_ref_key AS path_key FROM collection_objects WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(rows
        .iter()
        .map(map_object_row)
        .collect::<Result<Vec<_>, _>>()?)
}

pub async fn count_missing_mods_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    collection_id: &str,
) -> Result<usize, CollectionError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM collection_mods cm
        WHERE cm.collection_id = ?
          AND NOT EXISTS (
              SELECT 1
              FROM mods m
              WHERE m.game_id = ?
                AND m.folder_path_key = cm.mod_path_key
          )
        "#,
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_one(&mut *conn)
    .await?;

    Ok(count.max(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mod_mapper_rejects_missing_required_columns() {
        let test_db = crate::test_utils::init_test_db().await;
        let incomplete_row = sqlx::query("SELECT 'collection-1' AS collection_id")
            .fetch_one(&test_db.pool)
            .await
            .expect("query incomplete row");

        let error = map_mod_row(&incomplete_row).expect_err("required columns must not default");

        assert!(matches!(
            error,
            CollectionError::Db(message) if message.contains("no column found")
        ));
    }

    #[tokio::test]
    async fn mod_mapper_rejects_invalid_warnings_json() {
        let test_db = crate::test_utils::init_test_db().await;
        let corrupt_row = sqlx::query(
            r#"SELECT 'collection-1' AS collection_id, NULL AS mod_id,
               'AINOZ/Blue' AS mod_path, NULL AS mod_path_key,
               'object-1' AS effective_object_id,
               NULL AS display_name, NULL AS preview_path, NULL AS node_type,
               1 AS is_safe, NULL AS safety_source,
               'not-json' AS warnings_json"#,
        )
        .fetch_one(&test_db.pool)
        .await
        .expect("query corrupt row");

        let error = map_mod_row(&corrupt_row).expect_err("invalid JSON must not default");

        assert!(matches!(
            error,
            CollectionError::Db(message) if message.contains("Invalid warnings JSON")
        ));
    }
}
