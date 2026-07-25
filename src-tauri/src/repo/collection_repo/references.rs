//! Member-reference maintenance: path rewrites after a mod moves or an object
//! is renamed, plus the lookups the auto-heal path needs.

use sqlx::SqliteConnection;

use crate::domain::errors::CollectionError;

/// Auto-heal: update mod_path across all collections when a mod is moved/renamed.
pub async fn update_member_paths(
    conn: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<u64, CollectionError> {
    let result = sqlx::query(
        r#"UPDATE collection_mods
        SET mod_path = ?,
            mod_path_key = ?,
            object_id = COALESCE(?, object_id),
            preview_path = CASE
                WHEN preview_path = ? THEN ?
                ELSE preview_path
            END
        WHERE mod_path = ?"#,
    )
    .bind(new_mod_path)
    .bind(crate::common::path_key::folder_path_key(new_mod_path, None))
    .bind(new_object_id)
    .bind(old_mod_path)
    .bind(new_mod_path)
    .bind(old_mod_path)
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

/// `(collection_id, collection_name, mod_path)` for members whose path starts with `prefix`.
pub async fn find_mods_with_path_prefix(
    conn: &mut SqliteConnection,
    prefix: &str,
) -> Result<Vec<(String, String, String)>, CollectionError> {
    Ok(sqlx::query_as(
        r#"
        SELECT cm.collection_id, c.name, cm.mod_path
        FROM collection_mods cm
        INNER JOIN collections c ON c.id = cm.collection_id
        WHERE cm.mod_path LIKE ?
        "#,
    )
    .bind(format!("{prefix}%"))
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
            preview_path = CASE
                WHEN preview_path = ? THEN ?
                WHEN preview_path LIKE ? THEN REPLACE(preview_path, ?, ?)
                ELSE preview_path
            END
        WHERE collection_id = ? AND mod_path = ?
        "#,
    )
    .bind(new_path)
    .bind(crate::common::path_key::folder_path_key(new_path, None))
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

/// `(collection_id, collection_name)` of collections referencing the given mod path.
pub async fn get_references_by_mod_path(
    conn: &mut SqliteConnection,
    mod_path: &str,
) -> Result<Vec<(String, String)>, CollectionError> {
    Ok(sqlx::query_as(
        r#"
        SELECT DISTINCT c.id, c.name
        FROM collections c
        INNER JOIN collection_mods cm ON cm.collection_id = c.id
        WHERE cm.mod_path = ?
        ORDER BY c.name ASC, c.id ASC
        "#,
    )
    .bind(mod_path)
    .fetch_all(&mut *conn)
    .await?)
}

/// `(is_safe, mods_path)` context for signature recomputation.
pub async fn get_corridor_context(
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
