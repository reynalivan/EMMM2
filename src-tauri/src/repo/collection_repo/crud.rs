//! Collection row lifecycle: list, fetch, create, rename, delete.

use sqlx::{SqliteConnection, SqlitePool};

use super::mapping::row_to_collection;
use crate::common::path_key::canonical_name_key;
use crate::domain::collection::Collection;
use crate::domain::errors::CollectionError;

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
        r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                  last_active, signature, root_count, display_mod_count, created_at, updated_at
        FROM collections
        WHERE game_id = ?
        ORDER BY is_unsaved DESC, name ASC"#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_collection).collect())
}

/// List collections filtered by corridor and unsaved status.
///
/// Leaves `snapshot_json` `None` for the same reason as [`list_for_game`].
pub async fn list_for_corridor(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    include_unsaved: bool,
) -> Result<Vec<Collection>, CollectionError> {
    let unsaved_clause = if include_unsaved {
        ""
    } else {
        " AND c.is_unsaved = 0"
    };

    let query = format!(
        r#"SELECT c.id, c.game_id, c.name, c.name_key, c.is_safe, c.is_unsaved, c.is_last_unsaved,
                  c.last_active, c.signature, c.root_count, c.display_mod_count,
                  c.created_at, c.updated_at,
                  (SELECT COUNT(*) FROM collection_mods WHERE collection_id = c.id) +
                  (SELECT COUNT(*) FROM collection_objects WHERE collection_id = c.id) AS member_count_computed
        FROM collections c
        WHERE c.game_id = ? AND c.is_safe = ? {}
        ORDER BY c.name ASC"#,
        unsaved_clause
    );

    let rows = sqlx::query(&query)
        .bind(game_id)
        .bind(is_safe)
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(row_to_collection).collect())
}

/// Get a single collection by ID.
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Collection>, CollectionError> {
    let row = sqlx::query(
        r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                  last_active, snapshot_json, signature, root_count, display_mod_count, created_at, updated_at,
                  (SELECT COUNT(*) FROM collection_mods WHERE collection_id = collections.id) +
                  (SELECT COUNT(*) FROM collection_objects WHERE collection_id = collections.id) AS member_count_computed
        FROM collections
        WHERE id = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
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
    is_unsaved: bool,
) -> Result<Collection, CollectionError> {
    let name_key = canonical_name_key(name);
    let is_unsaved_i32 = if is_unsaved { 1i32 } else { 0i32 };

    // Duplicate check for named collections
    if !is_unsaved {
        let existing: Option<String> = sqlx::query_scalar(
            r#"SELECT id FROM collections
            WHERE game_id = ? AND name_key = ? AND is_safe = ? AND is_unsaved = 0"#,
        )
        .bind(game_id)
        .bind(&name_key)
        .bind(is_safe)
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
            return Err(CollectionError::DuplicateName {
                name: name.to_string(),
            });
        }
    }

    sqlx::query(
        r#"INSERT INTO collections (id, game_id, name, name_key, is_safe, is_unsaved, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
    )
    .bind(id)
    .bind(game_id)
    .bind(name)
    .bind(&name_key)
    .bind(is_safe)
    .bind(is_unsaved_i32)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.to_string() })
}

/// Delete a collection (CASCADE handles members).
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    let mut tx = pool.begin().await?;
    delete_tx(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_tx(conn: &mut SqliteConnection, id: &str) -> Result<(), CollectionError> {
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn find_unsaved_for_corridor(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    exclude_id: Option<&str>,
) -> Result<Option<Collection>, CollectionError> {
    let row = if let Some(excluded_id) = exclude_id {
        sqlx::query(
            r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                      last_active, snapshot_json, signature, root_count, display_mod_count, created_at, updated_at
               FROM collections
               WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1 AND id != ?
               ORDER BY updated_at DESC
               LIMIT 1"#,
        )
        .bind(game_id)
        .bind(is_safe)
        .bind(excluded_id)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                      last_active, snapshot_json, signature, root_count, display_mod_count, created_at, updated_at
               FROM collections
               WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1
               ORDER BY updated_at DESC
               LIMIT 1"#,
        )
        .bind(game_id)
        .bind(is_safe)
        .fetch_optional(pool)
        .await?
    };

    Ok(row.as_ref().map(row_to_collection))
}

pub async fn rename(
    pool: &SqlitePool,
    collection_id: &str,
    name: &str,
) -> Result<(), CollectionError> {
    sqlx::query("UPDATE collections SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(name)
        .bind(collection_id)
        .execute(pool)
        .await?;
    Ok(())
}
