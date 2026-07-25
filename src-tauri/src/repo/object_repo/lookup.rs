use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::types::*;

pub async fn get_runtime_descriptors(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ObjectRuntimeDescriptor>, sqlx::Error> {
    sqlx::query_as::<_, ObjectRuntimeDescriptor>(
        r#"
        SELECT
            id,
            name,
            folder_path,
            folder_path_key,
            matched_entry_key,
            matched_alias_name,
            object_type,
            thumbnail_path
        FROM objects
        WHERE game_id = ?
        ORDER BY name ASC
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn get_category_counts(
    pool: &SqlitePool,
    game_id: &str,
    _safe_mode: bool,
) -> Result<Vec<CategoryCount>, sqlx::Error> {
    // Phase 1 fix: always count ALL objects regardless of safe mode.
    // Category badges should show total counts; individual object counts
    // are zeroed for unsafe objects at the object level.
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT object_type, COUNT(*) as count FROM objects WHERE game_id = ");
    qb.push_bind(game_id);

    qb.push(" GROUP BY object_type ORDER BY object_type");

    qb.build_query_as::<CategoryCount>().fetch_all(pool).await
}

pub async fn get_characters_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    use sqlx::Row;
    let rows =
        sqlx::query("SELECT id, name FROM objects WHERE game_id = ? AND object_type = 'Character'")
            .bind(game_id)
            .fetch_all(pool)
            .await?;

    let mut result = Vec::new();
    for row in rows {
        result.push((row.try_get("id")?, row.try_get("name")?));
    }
    Ok(result)
}

pub async fn get_folder_path(pool: &SqlitePool, id: &str) -> Result<Option<String>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query("SELECT folder_path FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        Ok(r.try_get("folder_path").ok())
    } else {
        Ok(None)
    }
}

pub async fn get_rows_for_reconcile(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<ReconcileObjectRow>, sqlx::Error> {
    sqlx::query_as::<_, ReconcileObjectRow>(
        "SELECT id, folder_path, folder_path_key, status, object_type FROM objects WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn get_folder_path_conn(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let value: Option<Option<String>> =
        sqlx::query_scalar("SELECT folder_path FROM objects WHERE id = ? LIMIT 1")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(value.flatten())
}

pub async fn get_game_id(pool: &SqlitePool, id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT game_id FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_game_id_and_folder_path(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    sqlx::query_as("SELECT game_id, folder_path FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_object_type_by_id(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let value: Option<Option<String>> =
        sqlx::query_scalar("SELECT object_type FROM objects WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(value.flatten())
}

pub async fn get_object_id_by_folder_key(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path_key: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM objects WHERE game_id = ? AND folder_path_key = ? LIMIT 1")
        .bind(game_id)
        .bind(folder_path_key)
        .fetch_optional(&mut *conn)
        .await
}

pub async fn get_game_object_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<crate::domain::models::GameObject>, sqlx::Error> {
    sqlx::query_as::<_, crate::domain::models::GameObject>("SELECT * FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_mod_count_for_object(pool: &SqlitePool, id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}
