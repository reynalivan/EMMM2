//! Collection row lifecycle: list, fetch, create, rename, delete.

use sqlx::{SqliteConnection, SqlitePool};

use super::mapping::row_to_collection;
use crate::common::path_key::canonical_name_key;
use crate::domain::collection::Collection;
use crate::domain::errors::CollectionError;

pub struct CreateCollectionRow<'a> {
    pub id: &'a str,
    pub game_id: &'a str,
    pub name: &'a str,
    pub is_safe: bool,
}

/// List all collections for a game. Ordered by name.
///
/// `snapshot_json` is left `None`: it holds the full serialized projected state
/// (megabytes on a large library) and no list caller reads it. Fetch a single
/// collection by id when the snapshot is actually needed.
pub async fn list_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<Collection>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT c.id, c.game_id, c.name, c.name_key, c.is_safe,
                  0 AS is_draft, c.signature, c.display_mod_count,
                  c.created_at, c.updated_at
        FROM collections c
        LEFT JOIN collection_runtime_state runtime ON runtime.game_id = c.game_id
        WHERE c.game_id = ?
          AND (runtime.draft_collection_id IS NULL OR runtime.draft_collection_id != c.id)
        ORDER BY c.name ASC"#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_collection).collect())
}

/// Get a single collection by ID.
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Collection>, CollectionError> {
    let row = sqlx::query(
        r#"SELECT c.id, c.game_id, c.name, c.name_key, c.is_safe,
                  EXISTS(
                      SELECT 1 FROM collection_runtime_state runtime
                      WHERE runtime.game_id = c.game_id
                        AND runtime.draft_collection_id = c.id
                  ) AS is_draft,
                  c.snapshot_json, c.signature, c.display_mod_count,
                  c.created_at, c.updated_at
        FROM collections c
        WHERE c.id = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_collection))
}

pub async fn get_by_id_tx(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Collection>, CollectionError> {
    let row = sqlx::query(
        r#"SELECT c.id, c.game_id, c.name, c.name_key, c.is_safe,
                  EXISTS(
                      SELECT 1 FROM collection_runtime_state runtime
                      WHERE runtime.game_id = c.game_id
                        AND runtime.draft_collection_id = c.id
                  ) AS is_draft,
                  c.snapshot_json, c.signature, c.display_mod_count,
                  c.created_at, c.updated_at
        FROM collections c
        WHERE c.id = ?"#,
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.as_ref().map(row_to_collection))
}

/// Create a new collection.
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    is_safe: bool,
    is_draft: bool,
) -> Result<Collection, CollectionError> {
    let mut tx = pool.begin().await?;
    if is_draft {
        delete_unsaved_for_game_tx(&mut tx, game_id).await?;
    }
    create_tx(
        &mut tx,
        CreateCollectionRow {
            id,
            game_id,
            name,
            is_safe,
        },
    )
    .await?;
    if is_draft {
        crate::repo::collection::runtime::set_draft_tx(&mut tx, game_id, id, None).await?;
    }
    tx.commit().await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.to_string() })
}

pub async fn create_tx(
    conn: &mut SqliteConnection,
    collection: CreateCollectionRow<'_>,
) -> Result<(), CollectionError> {
    let name_key = canonical_name_key(collection.name);
    let duplicate_exists: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM collections
            WHERE game_id = ? AND name_key = ?
        )"#,
    )
    .bind(collection.game_id)
    .bind(&name_key)
    .fetch_one(&mut *conn)
    .await?;

    if duplicate_exists {
        return Err(CollectionError::DuplicateName {
            name: collection.name.to_string(),
        });
    }

    sqlx::query(
        r#"INSERT INTO collections (id, game_id, name, name_key, is_safe, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
    )
    .bind(collection.id)
    .bind(collection.game_id)
    .bind(collection.name)
    .bind(&name_key)
    .bind(collection.is_safe)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn delete_tx(conn: &mut SqliteConnection, id: &str) -> Result<(), CollectionError> {
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn delete_unsaved_for_game_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        "DELETE FROM collections WHERE id = (SELECT draft_collection_id FROM collection_runtime_state WHERE game_id = ?)",
    )
        .bind(game_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn update_safety_summary_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
    is_safe: bool,
) -> Result<(), CollectionError> {
    sqlx::query("UPDATE collections SET is_safe = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(is_safe)
        .bind(collection_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub struct MemberSafetySummary {
    pub contains_unsafe: bool,
    pub is_fully_classified: bool,
}

pub async fn member_safety_summary(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
) -> Result<MemberSafetySummary, CollectionError> {
    let rows = sqlx::query_as::<_, (bool, String)>(
        r#"SELECT
              CASE WHEN m.id IS NULL THEN COALESCE(cm.is_safe, 1)
                   ELSE COALESCE(m.is_safe, 1)
              END AS is_safe,
              CASE WHEN m.id IS NULL THEN COALESCE(cm.safety_source, 'unknown')
                   ELSE COALESCE(m.safety_source, 'unknown')
              END AS safety_source
            FROM collection_mods cm
            LEFT JOIN mods m
              ON m.game_id = ?
             AND (
                 (cm.mod_id IS NOT NULL AND m.id = cm.mod_id)
                 OR (cm.mod_path_key IS NOT NULL AND m.folder_path_key = cm.mod_path_key)
             )
            WHERE cm.collection_id = ?"#,
    )
    .bind(game_id)
    .bind(collection_id)
    .fetch_all(pool)
    .await?;
    let mut summary = MemberSafetySummary {
        contains_unsafe: false,
        is_fully_classified: true,
    };
    for (is_safe, source) in rows {
        let is_classified = source != crate::common::safety_constants::SAFETY_SOURCE_UNKNOWN;
        summary.is_fully_classified &= is_classified;
        summary.contains_unsafe |= is_classified && !is_safe;
    }
    Ok(summary)
}

pub async fn rename(
    pool: &SqlitePool,
    collection: &Collection,
    name: &str,
) -> Result<(), CollectionError> {
    let name_key = canonical_name_key(name);
    let rename_outcome = sqlx::query(
        r#"UPDATE collections
        SET name = ?, name_key = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND NOT EXISTS (
            SELECT 1 FROM collections duplicate
            WHERE duplicate.game_id = ?
              AND duplicate.name_key = ?
              AND duplicate.id != ?
        )"#,
    )
    .bind(name)
    .bind(&name_key)
    .bind(&collection.id)
    .bind(&collection.game_id)
    .bind(&name_key)
    .bind(&collection.id)
    .execute(pool)
    .await?;

    if rename_outcome.rows_affected() == 0 {
        if get_by_id(pool, &collection.id).await?.is_none() {
            return Err(CollectionError::NotFound {
                id: collection.id.clone(),
            });
        }
        return Err(CollectionError::DuplicateName {
            name: name.to_string(),
        });
    }

    Ok(())
}
