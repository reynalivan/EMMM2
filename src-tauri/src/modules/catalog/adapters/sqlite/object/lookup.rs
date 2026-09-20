use std::collections::HashSet;

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::types::*;
use crate::modules::catalog::domain::objects::{CategoryCount, ObjectRuntimeDescriptor};

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

/// Resolves only the candidate owner roots for one folder. Callers pass the
/// folder key and its ancestors, ordered from deepest to root, then preserve
/// the existing nearest-owner rule in memory.
pub async fn get_runtime_descriptors_for_folder_path_keys(
    pool: &SqlitePool,
    game_id: &str,
    folder_path_keys: &[String],
) -> Result<Vec<ObjectRuntimeDescriptor>, sqlx::Error> {
    if folder_path_keys.is_empty() {
        return Ok(Vec::new());
    }

    // Keep room for game_id below SQLite's common 999 bind-variable limit.
    const FOLDER_PATH_KEY_CHUNK_SIZE: usize = 900;
    let mut owners = Vec::new();
    for keys in folder_path_keys.chunks(FOLDER_PATH_KEY_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, name, folder_path, folder_path_key, matched_entry_key, matched_alias_name, object_type, thumbnail_path FROM objects WHERE game_id = ",
        );
        query.push_bind(game_id);
        query.push(" AND folder_path_key IN (");
        {
            let mut separated = query.separated(", ");
            for folder_path_key in keys {
                separated.push_bind(folder_path_key);
            }
        }
        query.push(") ORDER BY LENGTH(folder_path_key) DESC, name ASC");
        owners.extend(
            query
                .build_query_as::<ObjectRuntimeDescriptor>()
                .fetch_all(pool)
                .await?,
        );
    }
    Ok(owners)
}

pub async fn get_category_counts(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<CategoryCount>, sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT object_type, COUNT(*) as count FROM objects WHERE game_id = ");
    qb.push_bind(game_id);

    qb.push(" GROUP BY object_type ORDER BY object_type");

    qb.build_query_as::<CategoryCount>().fetch_all(pool).await
}

pub async fn get_rows_for_reconcile(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<ReconcileObjectRow>, sqlx::Error> {
    sqlx::query_as::<_, ReconcileObjectRow>(
        "SELECT id, name, folder_path, folder_path_key, status, filesystem_identity FROM objects WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await
}

/// Loads only object rows that can participate in a scoped projection.
///
/// Root keys cover normal create/update/prune work. Filesystem identities are
/// included as a second indexed lookup so a rename can still resolve the
/// pre-rename row after its path key changed.
pub async fn get_rows_for_reconcile_scope(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    root_keys: &[String],
    filesystem_identities: &[String],
) -> Result<Vec<ReconcileObjectRow>, sqlx::Error> {
    const LOOKUP_CHUNK_SIZE: usize = 900;
    let mut rows = Vec::new();
    let mut seen_ids = HashSet::new();

    for keys in root_keys.chunks(LOOKUP_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, name, folder_path, folder_path_key, status, filesystem_identity FROM objects WHERE game_id = ",
        );
        query.push_bind(game_id);
        query.push(" AND folder_path_key IN (");
        {
            let mut separated = query.separated(", ");
            for key in keys {
                separated.push_bind(key);
            }
        }
        query.push(")");
        for row in query
            .build_query_as::<ReconcileObjectRow>()
            .fetch_all(&mut *conn)
            .await?
        {
            if seen_ids.insert(row.id.clone()) {
                rows.push(row);
            }
        }
    }

    for identities in filesystem_identities.chunks(LOOKUP_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, name, folder_path, folder_path_key, status, filesystem_identity FROM objects WHERE game_id = ",
        );
        query.push_bind(game_id);
        query.push(" AND filesystem_identity IN (");
        {
            let mut separated = query.separated(", ");
            for identity in identities {
                separated.push_bind(identity);
            }
        }
        query.push(")");
        for row in query
            .build_query_as::<ReconcileObjectRow>()
            .fetch_all(&mut *conn)
            .await?
        {
            if seen_ids.insert(row.id.clone()) {
                rows.push(row);
            }
        }
    }

    Ok(rows)
}

pub async fn get_game_id_conn(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT game_id FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *conn)
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
) -> Result<Option<crate::modules::games::domain::models::GameObject>, sqlx::Error> {
    sqlx::query_as::<_, crate::modules::games::domain::models::GameObject>(
        "SELECT * FROM objects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_game_objects_by_ids(
    pool: &SqlitePool,
    game_id: &str,
    ids: &[String],
) -> Result<Vec<crate::modules::games::domain::models::GameObject>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    // SQLite's bind-variable limit is commonly 999; reserve room for game_id.
    const OBJECT_LOOKUP_CHUNK_SIZE: usize = 900;
    let mut objects = Vec::with_capacity(ids.len());
    for ids_chunk in ids.chunks(OBJECT_LOOKUP_CHUNK_SIZE) {
        let mut query = QueryBuilder::<Sqlite>::new("SELECT * FROM objects WHERE game_id = ");
        query.push_bind(game_id);
        query.push(" AND id IN (");
        {
            let mut ids_query = query.separated(", ");
            for id in ids_chunk {
                ids_query.push_bind(id);
            }
        }
        query.push(")");
        objects.extend(
            query
                .build_query_as::<crate::modules::games::domain::models::GameObject>()
                .fetch_all(pool)
                .await?,
        );
    }
    Ok(objects)
}

pub async fn get_mod_count_for_object(pool: &SqlitePool, id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}
