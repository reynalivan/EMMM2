//! The SQL `ensure_object_exists` is built from.
//!
//! Identity resolution -- which row an incoming folder *is*, and what a match
//! may overwrite -- is domain policy and lives in
//! `services::objects::reconcile`. This module only knows how to look a row up
//! and how to write one.

use crate::common::path_key::{canonical_name_key, folder_path_key};
use crate::domain::objects::EnsureObjectInput;

/// JSON sentinels the schema stores for "nothing set yet".
const EMPTY_TAGS: &str = "[]";
const EMPTY_METADATA: &str = "{}";

/// The four columns identity resolution reads.
///
/// The lookups used to select nine and read four; the other five were the
/// row's JSON blobs, fetched twice per object so a since-deleted read-back
/// could decide what was empty. SQL decides that now.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ObjectIdentityRow {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub object_type: String,
}

// Spelled out rather than assembled from a shared column list: sqlx caches
// prepared statements per connection, keyed by the SQL text, so query strings
// stay fixed. `object_type` is nullable and resolution treats a missing one as
// "Other". Both column lists must match `ObjectIdentityRow`.
const FIND_BY_NAME_KEY: &str = "SELECT id, name, folder_path, \
     COALESCE(object_type, 'Other') AS object_type \
     FROM objects WHERE game_id = ? AND name_key = ?";

const FIND_BY_FOLDER_KEY: &str = "SELECT id, name, folder_path, \
     COALESCE(object_type, 'Other') AS object_type \
     FROM objects WHERE game_id = ? AND folder_path_key = ?";

/// The object whose canonical name key matches, if any.
pub async fn find_by_name_key(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    name_key: &str,
) -> Result<Option<ObjectIdentityRow>, sqlx::Error> {
    sqlx::query_as(FIND_BY_NAME_KEY)
        .bind(game_id)
        .bind(name_key)
        .fetch_optional(conn)
        .await
}

/// The object sitting at this folder, if any.
pub async fn find_by_folder_key(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_key: &str,
) -> Result<Option<ObjectIdentityRow>, sqlx::Error> {
    sqlx::query_as(FIND_BY_FOLDER_KEY)
        .bind(game_id)
        .bind(folder_key)
        .fetch_optional(conn)
        .await
}

pub async fn update_object_location(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    folder_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ? WHERE id = ?")
        .bind(folder_path)
        .bind(folder_path_key(folder_path, None))
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn update_object_name(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE id = ?")
        .bind(name)
        .bind(canonical_name_key(name))
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn update_object_type(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    object_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET object_type = ? WHERE id = ?")
        .bind(object_type)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Fills the five columns a matched row left empty, leaving anything the user
/// already set alone.
///
/// One statement with fixed text, not five built with `format!`: sqlx caches
/// prepared statements per connection, so a per-column SQL string was a cache
/// miss and a fresh parse every time.
///
/// The `CASE` is what protects a value the user set: it only writes when the
/// column still holds the "nothing set" sentinel. An incoming sentinel used to
/// be filtered to NULL first, which changed nothing -- writing `[]` over `[]`
/// is the same as not writing -- so the guard is spelled once, here.
pub async fn backfill_empty_columns(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    input: &EnsureObjectInput<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE objects SET
             thumbnail_path = COALESCE(thumbnail_path, ?),
             hash_db        = COALESCE(hash_db, ?),
             custom_skins   = COALESCE(custom_skins, ?),
             tags           = CASE WHEN tags = ? THEN ? ELSE tags END,
             metadata       = CASE WHEN metadata = ? THEN ? ELSE metadata END
         WHERE id = ?",
    )
    .bind(input.db_thumbnail)
    .bind(input.db_hash_db_json)
    .bind(input.db_custom_skins_json)
    .bind(EMPTY_TAGS)
    .bind(input.db_tags_json)
    .bind(EMPTY_METADATA)
    .bind(input.db_metadata_json)
    .bind(id)
    .execute(conn)
    .await?;

    Ok(())
}

/// Insert a brand new object, enabled, and return its id.
pub async fn insert_object(
    conn: &mut sqlx::SqliteConnection,
    input: &EnsureObjectInput<'_>,
) -> Result<String, sqlx::Error> {
    let new_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)"
    )
    .bind(&new_id)
    .bind(input.game_id)
    .bind(input.obj_name)
    .bind(canonical_name_key(input.obj_name))
    .bind(input.folder_path)
    .bind(folder_path_key(input.folder_path, None))
    .bind(input.obj_type)
    .bind(input.db_thumbnail)
    .bind(input.db_tags_json)
    .bind(input.db_metadata_json)
    .bind(input.db_hash_db_json)
    .bind(input.db_custom_skins_json)
    .execute(conn)
    .await?;

    Ok(new_id)
}
