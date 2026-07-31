use super::types::*;
use crate::common::path_key::{canonical_name_key, folder_path_key};

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
        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());
        let existing_hash: Option<String> = row.try_get("hash_db").unwrap_or(None);
        let existing_skins: Option<String> = row.try_get("custom_skins").unwrap_or(None);

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

        if existing_type != obj_type && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET object_type = ? WHERE id = ?")
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_hash.is_none() && db_hash_db_json.is_some() {
            sqlx::query("UPDATE objects SET hash_db = ? WHERE id = ?")
                .bind(db_hash_db_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_skins.is_none() && db_custom_skins_json.is_some() {
            sqlx::query("UPDATE objects SET custom_skins = ? WHERE id = ?")
                .bind(db_custom_skins_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        return Ok(id);
    } else if let Some(row) = match_folder {
        let id: String = row.try_get("id").unwrap_or_default();
        let existing_fp: String = row.try_get("folder_path").unwrap_or_default();
        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());
        let existing_hash: Option<String> = row.try_get("hash_db").unwrap_or(None);
        let existing_skins: Option<String> = row.try_get("custom_skins").unwrap_or(None);

        if existing_fp != folder_path {
            sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&folder_key)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET name = ?, name_key = ?, object_type = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&name_key)
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        } else {
            sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&name_key)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_hash.is_none() && db_hash_db_json.is_some() {
            sqlx::query("UPDATE objects SET hash_db = ? WHERE id = ?")
                .bind(db_hash_db_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

        if existing_skins.is_none() && db_custom_skins_json.is_some() {
            sqlx::query("UPDATE objects SET custom_skins = ? WHERE id = ?")
                .bind(db_custom_skins_json)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }

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
