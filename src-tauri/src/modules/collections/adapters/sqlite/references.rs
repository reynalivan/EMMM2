//! Member-reference maintenance: path rewrites after a mod moves or an object
//! is renamed, plus the lookups the auto-heal path needs.

use sqlx::SqliteConnection;

use crate::shared::errors::CollectionError;

#[derive(Debug, Clone)]
pub struct CollectionMemberPathRow {
    pub collection_id: String,
    pub collection_name: String,
    pub mod_path: String,
}

pub async fn list_member_paths_for_game(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Vec<CollectionMemberPathRow>, CollectionError> {
    Ok(sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT cm.collection_id, c.name, cm.mod_path
           FROM collection_mods cm
           JOIN collections c ON c.id = cm.collection_id
           WHERE c.game_id = ?"#,
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(
        |(collection_id, collection_name, mod_path)| CollectionMemberPathRow {
            collection_id,
            collection_name,
            mod_path,
        },
    )
    .collect())
}

pub async fn stage_member_path(
    conn: &mut SqliteConnection,
    collection_id: &str,
    old_path: &str,
    staged_path: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        r#"UPDATE collection_mods
           SET mod_path = ?,
               mod_path_key = ?,
               preview_path = CASE
                   WHEN preview_path = ? THEN ?
                   WHEN preview_path LIKE ? ESCAPE '\' OR preview_path LIKE ? ESCAPE '\'
                       THEN ? || substr(preview_path, ?)
                   ELSE preview_path
               END
           WHERE collection_id = ? AND mod_path = ?"#,
    )
    .bind(staged_path)
    .bind(crate::shared::path_key::folder_path_key(staged_path, None))
    .bind(old_path)
    .bind(staged_path)
    .bind(format!("{}%", escape_like(&format!("{old_path}/"))))
    .bind(format!("{}%", escape_like(&format!("{old_path}\\"))))
    .bind(staged_path)
    .bind(old_path.chars().count() as i64 + 1)
    .bind(collection_id)
    .bind(old_path)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn finalize_staged_member_path(
    conn: &mut SqliteConnection,
    collection_id: &str,
    staged_path: &str,
    final_path: &str,
) -> Result<(), CollectionError> {
    let final_key = crate::shared::path_key::folder_path_key(final_path, None);
    let destination_paths: Vec<String> = sqlx::query_scalar(
        r#"SELECT mod_path FROM collection_mods
           WHERE collection_id = ? AND mod_path <> ?"#,
    )
    .bind(collection_id)
    .bind(staged_path)
    .fetch_all(&mut *conn)
    .await?;
    let matching_destination = destination_paths
        .iter()
        .find(|path| crate::shared::path_key::folder_path_key(path, None) == final_key);
    if let Some(destination_path) = matching_destination {
        // The physical source identity won the replacement, so its durable
        // preview/node/warning fields must win too. Remove the obsolete
        // destination membership, then finalize the staged source below.
        sqlx::query("DELETE FROM collection_mods WHERE collection_id = ? AND mod_path = ?")
            .bind(collection_id)
            .bind(destination_path)
            .execute(&mut *conn)
            .await?;
    }

    sqlx::query(
        r#"UPDATE collection_mods
           SET mod_path = ?,
               mod_path_key = ?,
               object_ref_key = ?,
               preview_path = CASE
                   WHEN preview_path = ? THEN ?
                   WHEN preview_path LIKE ? ESCAPE '\' OR preview_path LIKE ? ESCAPE '\'
                       THEN ? || substr(preview_path, ?)
                   ELSE preview_path
               END
           WHERE collection_id = ? AND mod_path = ?"#,
    )
    .bind(final_path)
    .bind(&final_key)
    .bind(final_key.split('/').next().unwrap_or_default())
    .bind(staged_path)
    .bind(final_path)
    .bind(format!("{}%", escape_like(&format!("{staged_path}/"))))
    .bind(format!("{}%", escape_like(&format!("{staged_path}\\"))))
    .bind(final_path)
    .bind(staged_path.chars().count() as i64 + 1)
    .bind(collection_id)
    .bind(staged_path)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn stage_object_reference(
    conn: &mut SqliteConnection,
    game_id: &str,
    old_ref_key: &str,
    staged_ref_key: &str,
) -> Result<Vec<(String, String)>, CollectionError> {
    let references = sqlx::query_as(
        r#"SELECT c.id, c.name
           FROM collection_objects co
           JOIN collections c ON c.id = co.collection_id
           WHERE c.game_id = ? AND co.object_ref_key = ?"#,
    )
    .bind(game_id)
    .bind(old_ref_key)
    .fetch_all(&mut *conn)
    .await?;
    sqlx::query(
        r#"UPDATE collection_objects
           SET object_ref_key = ?
           WHERE object_ref_key = ?
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_objects.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(staged_ref_key)
    .bind(old_ref_key)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(references)
}

pub async fn finalize_staged_object_reference(
    conn: &mut SqliteConnection,
    game_id: &str,
    staged_ref_key: &str,
    final_ref_key: &str,
    final_display_name: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        r#"DELETE FROM collection_objects
           WHERE object_ref_key = ?
             AND EXISTS (
                 SELECT 1 FROM collection_objects source
                 WHERE source.collection_id = collection_objects.collection_id
                   AND source.object_ref_key = ?
             )
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_objects.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(final_ref_key)
    .bind(staged_ref_key)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        r#"UPDATE collection_objects
           SET object_ref_key = ?, object_display_name = ?
           WHERE object_ref_key = ?
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_objects.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(final_ref_key)
    .bind(final_display_name)
    .bind(staged_ref_key)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn detach_mod_runtime_id(
    conn: &mut SqliteConnection,
    game_id: &str,
    mod_id: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        r#"UPDATE collection_mods
           SET mod_id = NULL
           WHERE mod_id = ?
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_mods.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(mod_id)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn rebind_object_references(
    conn: &mut SqliteConnection,
    game_id: &str,
    object_ref_key: &str,
    object_id: &str,
) -> Result<(), CollectionError> {
    for table in ["collection_objects", "collection_mods"] {
        let sql = format!(
            "UPDATE {table} SET object_id = ? WHERE object_ref_key = ? AND EXISTS (SELECT 1 FROM collections c WHERE c.id = {table}.collection_id AND c.game_id = ?)"
        );
        sqlx::query(&sql)
            .bind(object_id)
            .bind(object_ref_key)
            .bind(game_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn rebind_mod_references(
    conn: &mut SqliteConnection,
    game_id: &str,
    exact_path_key: &str,
    logical_path_key: &str,
    mod_id: &str,
    object_id: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        r#"UPDATE collection_mods
           SET mod_id = ?, object_id = ?
           WHERE (mod_path_key = ? OR mod_path_key = ?)
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_mods.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(mod_id)
    .bind(object_id)
    .bind(exact_path_key)
    .bind(logical_path_key)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Auto-heal: update mod_path across all collections when a mod is moved/renamed.
pub async fn update_member_paths(
    conn: &mut SqliteConnection,
    game_id: &str,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<u64, CollectionError> {
    let new_path_key = crate::shared::path_key::folder_path_key(new_mod_path, None);
    // Collection membership is a logical set. Compare canonical keys in Rust
    // so legacy rows with a NULL/stale `mod_path_key` still merge safely when
    // an offline rename A -> B replaces a deleted B.
    let source_collections: Vec<String> = sqlx::query_scalar(
        r#"SELECT cm.collection_id
           FROM collection_mods cm
           JOIN collections c ON c.id = cm.collection_id
           WHERE c.game_id = ? AND cm.mod_path = ?"#,
    )
    .bind(game_id)
    .bind(old_mod_path)
    .fetch_all(&mut *conn)
    .await?;
    let mut merged = 0;
    for collection_id in source_collections {
        let sibling_paths: Vec<String> = sqlx::query_scalar(
            "SELECT mod_path FROM collection_mods WHERE collection_id = ? AND mod_path <> ?",
        )
        .bind(&collection_id)
        .bind(old_mod_path)
        .fetch_all(&mut *conn)
        .await?;
        if let Some(destination_path) = sibling_paths
            .iter()
            .find(|path| crate::shared::path_key::folder_path_key(path, None) == new_path_key)
        {
            merged +=
                sqlx::query("DELETE FROM collection_mods WHERE collection_id = ? AND mod_path = ?")
                    .bind(&collection_id)
                    .bind(destination_path)
                    .execute(&mut *conn)
                    .await?
                    .rows_affected();
        }
    }
    let result = sqlx::query(
        r#"UPDATE collection_mods
        SET mod_path = ?,
            mod_path_key = ?,
            object_ref_key = ?,
            object_id = COALESCE(?, object_id),
            preview_path = CASE
                WHEN preview_path = ? THEN ?
                ELSE preview_path
            END
        WHERE mod_path = ?
          AND EXISTS (
              SELECT 1 FROM collections c
              WHERE c.id = collection_mods.collection_id AND c.game_id = ?
          )"#,
    )
    .bind(new_mod_path)
    .bind(new_path_key)
    .bind(
        crate::shared::path_key::folder_path_key(new_mod_path, None)
            .split('/')
            .next()
            .unwrap_or_default(),
    )
    .bind(new_object_id)
    .bind(old_mod_path)
    .bind(new_mod_path)
    .bind(old_mod_path)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;

    Ok(merged + result.rows_affected())
}

pub async fn update_member_mod_id_for_path(
    conn: &mut SqliteConnection,
    game_id: &str,
    mod_path: &str,
    mod_id: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        r#"UPDATE collection_mods
           SET mod_id = ?
           WHERE mod_path = ?
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_mods.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(mod_id)
    .bind(mod_path)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// `(collection_id, collection_name, mod_path)` for members whose path starts with `prefix`.
pub async fn find_mods_with_path_prefix(
    conn: &mut SqliteConnection,
    game_id: &str,
    prefix: &str,
) -> Result<Vec<(String, String, String)>, CollectionError> {
    Ok(sqlx::query_as(
        r#"
        SELECT cm.collection_id, c.name, cm.mod_path
        FROM collection_mods cm
        INNER JOIN collections c ON c.id = cm.collection_id
        WHERE c.game_id = ? AND cm.mod_path LIKE ? ESCAPE '\'
        "#,
    )
    .bind(game_id)
    .bind(format!("{}%", escape_like(prefix)))
    .fetch_all(&mut *conn)
    .await?)
}

/// Rewrite one member path (and matching preview paths) after an object rename.
pub async fn rewrite_member_path(
    conn: &mut SqliteConnection,
    collection_id: &str,
    old_path: &str,
    new_path: &str,
    old_sep: &str,
    new_sep: &str,
) -> Result<(), CollectionError> {
    sqlx::query(
        r#"
        UPDATE collection_mods
        SET
            mod_path = ?,
            mod_path_key = ?,
            object_ref_key = ?,
            preview_path = CASE
                WHEN preview_path = ? THEN ?
                WHEN preview_path LIKE ? THEN REPLACE(preview_path, ?, ?)
                ELSE preview_path
            END
        WHERE collection_id = ? AND mod_path = ?
        "#,
    )
    .bind(new_path)
    .bind(crate::shared::path_key::folder_path_key(new_path, None))
    .bind(
        crate::shared::path_key::folder_path_key(new_path, None)
            .split('/')
            .next()
            .unwrap_or_default(),
    )
    .bind(old_path)
    .bind(new_path)
    .bind(format!("{}%", old_sep))
    .bind(old_sep)
    .bind(new_sep)
    .bind(collection_id)
    .bind(old_path)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn rewrite_object_references(
    conn: &mut SqliteConnection,
    game_id: &str,
    old_ref_key: &str,
    new_ref_key: &str,
    new_display_name: &str,
) -> Result<Vec<(String, String)>, CollectionError> {
    let references = sqlx::query_as(
        r#"SELECT c.id, c.name
           FROM collection_objects co
           JOIN collections c ON c.id = co.collection_id
           WHERE c.game_id = ? AND co.object_ref_key = ?"#,
    )
    .bind(game_id)
    .bind(old_ref_key)
    .fetch_all(&mut *conn)
    .await?;
    sqlx::query(
        r#"UPDATE collection_objects
           SET object_ref_key = ?, object_display_name = ?
           WHERE object_ref_key = ?
             AND EXISTS (
                 SELECT 1 FROM collections c
                 WHERE c.id = collection_objects.collection_id AND c.game_id = ?
             )"#,
    )
    .bind(new_ref_key)
    .bind(new_display_name)
    .bind(old_ref_key)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(references)
}

/// `(collection_id, collection_name)` of collections referencing the given mod path.
pub async fn get_references_by_mod_path(
    conn: &mut SqliteConnection,
    game_id: &str,
    mod_path: &str,
) -> Result<Vec<(String, String)>, CollectionError> {
    Ok(sqlx::query_as(
        r#"
        SELECT DISTINCT c.id, c.name
        FROM collections c
        INNER JOIN collection_mods cm ON cm.collection_id = c.id
        WHERE c.game_id = ? AND cm.mod_path = ?
        ORDER BY c.name ASC, c.id ASC
        "#,
    )
    .bind(game_id)
    .bind(mod_path)
    .fetch_all(&mut *conn)
    .await?)
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// `(is_safe, mods_path)` context for signature recomputation.
pub async fn get_projection_context(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(i32, Option<String>), CollectionError> {
    Ok(sqlx::query_as(
        r#"
        SELECT c.is_safe, g.mods_path
        FROM collections c
        LEFT JOIN games g ON g.id = c.game_id
        WHERE c.id = ?
        "#,
    )
    .bind(collection_id)
    .fetch_one(&mut *conn)
    .await?)
}
