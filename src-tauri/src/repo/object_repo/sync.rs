use super::types::*;
use crate::common::path_key::{canonical_name_key, folder_path_key};

/// JSON sentinels the schema stores for "nothing set yet".
const EMPTY_TAGS: &str = "[]";
const EMPTY_METADATA: &str = "{}";

/// Fills the five columns a matched row left empty, leaving anything the user
/// already set alone.
///
/// One statement with fixed text, not five built with `format!`: sqlx caches
/// prepared statements per connection, so a per-column SQL string was a cache
/// miss and a fresh parse every time. SQL decides what is empty, so the row's
/// current values no longer have to be read back first.
async fn backfill_empty_columns(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    input: &EnsureObjectInput<'_>,
) -> Result<(), sqlx::Error> {
    // The `CASE` is what protects a value the user set: it only writes when the
    // column still holds the "nothing set" sentinel. An incoming sentinel used
    // to be filtered to NULL first, which changed nothing -- writing `[]` over
    // `[]` is the same as not writing -- so the guard is spelled once, here.
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
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn ensure_object_exists(
    conn: &mut sqlx::SqliteConnection,
    input: EnsureObjectInput<'_>,
    new_objects_count: &mut usize,
) -> Result<String, sqlx::Error> {
    use sqlx::Row;
    let EnsureObjectInput {
        game_id,
        folder_path,
        obj_name,
        obj_type,
        db_thumbnail,
        db_tags_json,
        db_metadata_json,
        db_hash_db_json,
        db_custom_skins_json,
        source: _,
    } = input;
    let name_key = canonical_name_key(obj_name);
    let folder_key = folder_path_key(folder_path, None);

    let match_name = sqlx::query(
        "SELECT id, name, folder_path, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins
         FROM objects
         WHERE game_id = ? AND name_key = ?",
    )
    .bind(game_id)
    .bind(&name_key)
    .fetch_optional(&mut *conn)
    .await
    ?;

    let match_folder = sqlx::query(
        "SELECT id, name, folder_path, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins
         FROM objects
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(game_id)
    .bind(&folder_key)
    .fetch_optional(&mut *conn)
    .await
    ?;

    if let Some(row) = match_name {
        let id: String = row.try_get("id").unwrap_or_default();
        let existing_name: String = row.try_get("name").unwrap_or_default();
        let existing_fp: String = row.try_get("folder_path").unwrap_or_default();
        let existing_type: String = row
            .try_get("object_type")
            .unwrap_or_else(|_| "Other".to_string());
        let has_folder_conflict = match_folder
            .as_ref()
            .and_then(|folder_row| folder_row.try_get::<String, _>("id").ok())
            .is_some_and(|folder_id| folder_id != id);

        if existing_fp != folder_path && !has_folder_conflict {
            sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&folder_key)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_name != obj_name {
            sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&name_key)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_type != obj_type && input.source.type_is_authoritative() {
            sqlx::query("UPDATE objects SET object_type = ? WHERE id = ?")
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        backfill_empty_columns(&mut *conn, &id, &input).await?;

        return Ok(id);
    }

    if let Some(row) = match_folder {
        let id: String = row.try_get("id").unwrap_or_default();
        let existing_fp: String = row.try_get("folder_path").unwrap_or_default();

        if existing_fp != folder_path {
            sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&folder_key)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE id = ?")
            .bind(obj_name)
            .bind(&name_key)
            .bind(&id)
            .execute(&mut *conn)
            .await?;

        if input.source.type_is_authoritative() {
            sqlx::query("UPDATE objects SET object_type = ? WHERE id = ?")
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        backfill_empty_columns(&mut *conn, &id, &input).await?;

        return Ok(id);
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)"
    )
    .bind(&new_id)
    .bind(game_id)
    .bind(obj_name)
    .bind(&name_key)
    .bind(folder_path)
    .bind(&folder_key)
    .bind(obj_type)
    .bind(db_thumbnail)
    .bind(db_tags_json)
    .bind(db_metadata_json)
    .bind(db_hash_db_json)
    .bind(db_custom_skins_json)
    .execute(&mut *conn)
    .await
    ?;

    *new_objects_count += 1;
    Ok(new_id)
}
