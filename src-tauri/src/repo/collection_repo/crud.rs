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
    pub is_unsaved: bool,
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

pub async fn list_named_for_corridor(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Vec<Collection>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT c.id, c.game_id, c.name, c.name_key, c.is_safe, c.is_unsaved, c.is_last_unsaved,
                  c.last_active, c.signature, c.root_count, c.display_mod_count,
                  c.created_at, c.updated_at
        FROM collections c
        WHERE c.game_id = ? AND c.is_safe = ? AND c.is_unsaved = 0
        ORDER BY c.name ASC"#,
    )
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
                  last_active, snapshot_json, signature, root_count, display_mod_count, created_at, updated_at
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
    let mut tx = pool.begin().await?;
    create_tx(
        &mut tx,
        CreateCollectionRow {
            id,
            game_id,
            name,
            is_safe,
            is_unsaved,
        },
    )
    .await?;
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
    if !collection.is_unsaved {
        let duplicate_exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(
                SELECT 1 FROM collections
                WHERE game_id = ? AND name_key = ? AND is_safe = ? AND is_unsaved = 0
            )"#,
        )
        .bind(collection.game_id)
        .bind(&name_key)
        .bind(collection.is_safe)
        .fetch_one(&mut *conn)
        .await?;

        if duplicate_exists {
            return Err(CollectionError::DuplicateName {
                name: collection.name.to_string(),
            });
        }
    }

    sqlx::query(
        r#"INSERT INTO collections (id, game_id, name, name_key, is_safe, is_unsaved, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
    )
    .bind(collection.id)
    .bind(collection.game_id)
    .bind(collection.name)
    .bind(&name_key)
    .bind(collection.is_safe)
    .bind(collection.is_unsaved)
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

pub async fn find_unsaved_id_for_corridor_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    is_safe: bool,
    exclude_id: &str,
) -> Result<Option<String>, CollectionError> {
    sqlx::query_scalar(
        "SELECT id FROM collections WHERE game_id = ? AND is_safe = ? AND is_unsaved = 1 AND id != ? LIMIT 1",
    )
    .bind(game_id)
    .bind(is_safe)
    .bind(exclude_id)
    .fetch_optional(conn)
    .await
    .map_err(CollectionError::from)
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
              AND duplicate.is_safe = ?
              AND duplicate.is_unsaved = 0
              AND duplicate.id != ?
        )"#,
    )
    .bind(name)
    .bind(&name_key)
    .bind(&collection.id)
    .bind(&collection.game_id)
    .bind(&name_key)
    .bind(collection.is_safe)
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
