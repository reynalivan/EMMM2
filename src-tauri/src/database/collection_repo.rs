use crate::services::collections::types::{
    CanonicalCollectionSnapshot, Collection, CollectionDetails, CollectionObjectState,
    CollectionPreviewMod, ModState,
};
use crate::services::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::path_key::{
    canonical_collection_path_key, collection_mod_path_key, collection_name_key,
};
use sqlx::{QueryBuilder, Sqlite, SqliteConnection, SqlitePool};
use std::collections::HashMap;

pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<Vec<Collection>, sqlx::Error> {
    let sql = r#"
        SELECT c.id, c.name, c.game_id, c.is_safe_context,
            COALESCE(c.root_count, 0) as member_count,
            COALESCE(c.is_last_unsaved, 0) as is_last_unsaved
        FROM collections c
        WHERE c.game_id = ? AND c.is_safe_context = ?
        ORDER BY c.is_last_unsaved DESC, c.name
    "#;

    let rows = sqlx::query_as::<_, (String, String, String, bool, i64, bool)>(sql)
        .bind(game_id)
        .bind(safe_mode_enabled)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, gid, safe, count, is_last_unsaved)| Collection {
            id,
            name,
            game_id: gid,
            is_safe_context: safe,
            member_count: count as usize,
            is_last_unsaved,
        })
        .collect())
}

pub async fn check_collection_exists(
    conn: &mut SqliteConnection,
    game_id: &str,
    name: &str,
    is_safe_context: bool,
) -> Result<bool, sqlx::Error> {
    let name_key = collection_name_key(name);
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM collections WHERE game_id = ? AND name_key = ? AND is_safe_context = ?",
    )
    .bind(game_id)
    .bind(name_key)
    .bind(is_safe_context)
    .fetch_optional(conn)
    .await?;

    Ok(existing.is_some())
}

pub async fn insert_collection(
    conn: &mut SqliteConnection,
    id: &str,
    name: &str,
    game_id: &str,
    is_safe_context: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO collections (id, name, name_key, game_id, is_safe_context) VALUES (?, ?, ?, ?, ?)",
    )
        .bind(id)
        .bind(name)
        .bind(collection_name_key(name))
        .bind(game_id)
        .bind(is_safe_context)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn upsert_collection_snapshot(
    conn: &mut SqliteConnection,
    collection_id: &str,
    snapshot: &CanonicalCollectionSnapshot,
    signature: &str,
) -> Result<(), sqlx::Error> {
    let snapshot_json = serde_json::to_string(snapshot).map_err(|error| {
        sqlx::Error::Protocol(format!("Failed to serialize collection snapshot: {error}"))
    })?;

    sqlx::query(
        "UPDATE collections SET snapshot_json = ?, signature = ?, root_count = ? WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind(signature)
    .bind(snapshot.summary.root_count as i64)
    .bind(collection_id)
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn get_collection_snapshot(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Option<CanonicalCollectionSnapshot>, sqlx::Error> {
    let snapshot_json = sqlx::query_scalar::<_, Option<String>>(
        "SELECT snapshot_json FROM collections WHERE id = ?",
    )
    .bind(collection_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    let Some(snapshot_json) = snapshot_json else {
        return Ok(None);
    };

    let snapshot =
        serde_json::from_str::<CanonicalCollectionSnapshot>(&snapshot_json).map_err(|error| {
            sqlx::Error::Protocol(format!(
                "Failed to deserialize collection snapshot: {error}"
            ))
        })?;

    Ok(Some(snapshot))
}

pub async fn get_mod_paths_for_ids(
    conn: &mut SqliteConnection,
    mod_ids: &[String],
) -> Result<HashMap<String, String>, sqlx::Error> {
    if mod_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id, folder_path FROM mods WHERE id IN (");
    let mut sep = qb.separated(", ");
    for id in mod_ids {
        sep.push_bind(id);
    }
    qb.push(")");

    let rows: Vec<(String, String)> = qb.build_query_as().fetch_all(conn).await?;

    let mut paths = HashMap::new();
    for (id, path) in rows {
        paths.insert(id, path);
    }
    Ok(paths)
}

pub async fn get_mod_paths_for_ids_pool(
    pool: &SqlitePool,
    mod_ids: &[String],
) -> Result<HashMap<String, String>, sqlx::Error> {
    if mod_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id, folder_path FROM mods WHERE id IN (");
    let mut sep = qb.separated(", ");
    for id in mod_ids {
        sep.push_bind(id);
    }
    qb.push(")");

    let rows: Vec<(String, String)> = qb.build_query_as().fetch_all(pool).await?;

    let mut paths = HashMap::new();
    for (id, path) in rows {
        paths.insert(id, path);
    }
    Ok(paths)
}

pub async fn get_collection_name(
    conn: &mut SqliteConnection,
    id: &str,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT name FROM collections WHERE id = ? AND game_id = ?")
        .bind(id)
        .bind(game_id)
        .fetch_optional(conn)
        .await
}

pub async fn update_collection_name(
    conn: &mut SqliteConnection,
    id: &str,
    game_id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE collections SET name = ?, name_key = ? WHERE id = ? AND game_id = ?")
        .bind(name)
        .bind(collection_name_key(name))
        .bind(id)
        .bind(game_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn update_collection_safe_context(
    conn: &mut SqliteConnection,
    id: &str,
    game_id: &str,
    is_safe: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE collections SET is_safe_context = ? WHERE id = ? AND game_id = ?")
        .bind(is_safe)
        .bind(id)
        .bind(game_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_collection(
    conn: &mut SqliteConnection,
    id: &str,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collections WHERE id = ? AND game_id = ?")
        .bind(id)
        .bind(game_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_collection_details(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<Option<CollectionDetails>, sqlx::Error> {
    let coll_row: Option<(String, String, String, bool, bool, i64)> = sqlx::query_as(
        "SELECT id, name, game_id, is_safe_context, COALESCE(is_last_unsaved, 0), COALESCE(root_count, 0) FROM collections WHERE id = ? AND game_id = ?",
    )
    .bind(id)
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    if let Some((cid, name, gid, safe, is_last_unsaved, root_count)) = coll_row {
        let mod_ids = get_collection_root_mod_ids_for_game(pool, id, game_id).await?;
        let object_states = get_collection_object_states(pool, id).await?;

        Ok(Some(CollectionDetails {
            collection: Collection {
                id: cid,
                name,
                game_id: gid,
                is_safe_context: safe,
                member_count: root_count as usize,
                is_last_unsaved,
            },
            mod_ids,
            object_states,
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_collection_object_states(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionObjectState>, sqlx::Error> {
    if let Some(snapshot) = get_collection_snapshot(pool, collection_id).await? {
        return Ok(snapshot
            .object_states
            .into_iter()
            .map(|state| CollectionObjectState {
                object_id: state.object_id,
                is_enabled: state.is_enabled,
            })
            .collect());
    }

    sqlx::query_as::<_, (String, bool)>(
        "SELECT object_id, is_enabled FROM collection_object_states WHERE collection_id = ? ORDER BY object_id",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(object_id, is_enabled)| CollectionObjectState {
                object_id,
                is_enabled,
            })
            .collect()
    })
}

pub async fn delete_collection_object_states(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collection_object_states WHERE collection_id = ?")
        .bind(collection_id)
        .execute(conn)
        .await?;
    Ok(())
}

// ── Required by apply.rs ────────────────────────────────────

pub async fn get_collection_items_with_missing_mods(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<Vec<(String, Option<String>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT ci.mod_id, ci.mod_path FROM collection_items ci WHERE ci.collection_id = ? AND ci.mod_id NOT IN (SELECT id FROM mods WHERE game_id = ?)",
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn get_nested_collection_items(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT mod_path FROM collection_nested_items WHERE collection_id = ?")
        .bind(collection_id)
        .fetch_all(pool)
        .await
}

pub async fn get_mod_ids_for_collection_in_game(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT ci.mod_id FROM collection_items ci JOIN mods m ON m.id = ci.mod_id WHERE ci.collection_id = ? AND m.game_id = ?",
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_snapshot_collection(
    conn: &mut SqliteConnection,
    game_id: &str,
    is_safe_context: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM collections WHERE game_id = ? AND is_last_unsaved = 1 AND is_safe_context = ?",
    )
    .bind(game_id)
    .bind(is_safe_context)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn insert_snapshot_collection(
    conn: &mut SqliteConnection,
    snapshot_id: &str,
    name: &str,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO collections (id, name, name_key, game_id, is_safe_context, is_last_unsaved) VALUES (?, ?, ?, ?, ?, 1)",
    )
        .bind(snapshot_id)
        .bind(name)
        .bind(collection_name_key(name))
        .bind(game_id)
        .bind(safe_mode_enabled)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_mod_states_by_ids(
    pool: &SqlitePool,
    game_id: &str,
    ids: &[String],
) -> Result<Vec<ModState>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id, folder_path, status, object_id FROM mods WHERE game_id = ");
    qb.push_bind(game_id).push(" AND id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    qb.push(")");

    let rows: Vec<(String, String, String, Option<String>)> =
        qb.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, folder_path, status, object_id)| ModState {
            id,
            folder_path,
            status,
            object_id,
        })
        .collect())
}

pub async fn get_collection_preview_mods_by_ids(
    pool: &SqlitePool,
    game_id: &str,
    ids: &[String],
) -> Result<Vec<CollectionPreviewMod>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
        "SELECT m.id, m.actual_name, m.folder_path, COALESCE(m.is_safe, 1), m.object_id, o.name, o.object_type FROM mods m LEFT JOIN objects o ON m.object_id = o.id WHERE m.game_id = ",
    );
    qb.push_bind(game_id).push(" AND m.id IN (");

    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    qb.push(") ORDER BY m.actual_name, m.folder_path");

    let rows: Vec<(
        String,
        String,
        String,
        bool,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = qb.build_query_as().fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, actual_name, folder_path, is_safe, object_id, object_name, object_type)| {
                CollectionPreviewMod {
                    id,
                    actual_name,
                    folder_path,
                    is_safe,
                    object_id,
                    object_name,
                    object_type,
                    node_type: None,
                }
            },
        )
        .collect())
}

pub async fn batch_update_mods_status_and_path(
    pool: &SqlitePool,
    mods_path: Option<&str>,
    updates: &[(String, String, String, Option<String>)], // (id, status, folder_path, disabled_reason)
) -> Result<(), sqlx::Error> {
    if updates.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (id, status, folder_path, disabled_reason) in updates {
        sqlx::query(
            "UPDATE mods SET status = ?, folder_path = ?, folder_path_key = ?, disabled_reason = ? WHERE id = ?",
        )
        .bind(status)
        .bind(folder_path)
        .bind(collection_mod_path_key(folder_path, mods_path))
        .bind(disabled_reason)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn get_enabled_mod_id_and_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'")
        .bind(game_id)
        .fetch_all(pool)
        .await
}

/// Corridor-aware: only returns enabled mods matching the given `is_safe` context.
pub async fn get_enabled_mod_id_and_paths_for_corridor(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT m.id, m.folder_path FROM mods m
           LEFT JOIN objects o ON m.object_id = o.id
           WHERE m.game_id = ? AND m.status = 'ENABLED'
             AND (
                COALESCE(m.is_safe, 1) = ?
                OR COALESCE(m.corridor_source, ?) IN (?, ?)
             )"#,
    )
    .bind(game_id)
    .bind(is_safe)
    .bind(CORRIDOR_SOURCE_UNKNOWN)
    .bind(CORRIDOR_SOURCE_MANUAL)
    .bind(CORRIDOR_SOURCE_UNKNOWN)
    .fetch_all(pool)
    .await
}

pub async fn get_collection_root_mod_ids_for_game(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id).await?;
    let root_keys: Vec<String> = get_collection_roots(pool, collection_id)
        .await?
        .into_iter()
        .filter_map(|root| canonical_collection_path_key(&root.folder_path, mods_path.as_deref()))
        .collect();
    if root_keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT DISTINCT id FROM mods WHERE game_id = ");
    qb.push_bind(game_id).push(" AND (");

    for (index, root_key) in root_keys.iter().enumerate() {
        if index > 0 {
            qb.push(" OR ");
        }
        qb.push("(folder_path_key = ")
            .push_bind(root_key)
            .push(" OR folder_path_key LIKE ")
            .push_bind(format!("{root_key}/%"))
            .push(")");
    }
    qb.push(") ORDER BY id");

    qb.build_query_scalar::<String>().fetch_all(pool).await
}

pub async fn get_collection_root_mod_paths(
    pool: &SqlitePool,
    collection_id: &str,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id).await?;
    let root_keys: Vec<String> = get_collection_roots(pool, collection_id)
        .await?
        .into_iter()
        .filter_map(|root| canonical_collection_path_key(&root.folder_path, mods_path.as_deref()))
        .collect();
    if root_keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT DISTINCT folder_path FROM mods WHERE game_id = ");
    qb.push_bind(game_id).push(" AND (");

    for (index, root_key) in root_keys.iter().enumerate() {
        if index > 0 {
            qb.push(" OR ");
        }
        qb.push("(folder_path_key = ")
            .push_bind(root_key)
            .push(" OR folder_path_key LIKE ")
            .push_bind(format!("{root_key}/%"))
            .push(")");
    }
    qb.push(") ORDER BY folder_path");

    qb.build_query_scalar::<String>().fetch_all(pool).await
}

pub async fn get_enabled_mod_ids_for_object_ids(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode: bool,
    object_ids: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    if object_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id FROM mods WHERE game_id = ");
    qb.push_bind(game_id)
        .push(" AND status = 'ENABLED' AND (COALESCE(is_safe, 1) = ")
        .push_bind(safe_mode)
        .push(" OR COALESCE(corridor_source, ")
        .push_bind(CORRIDOR_SOURCE_UNKNOWN)
        .push(") IN (")
        .push_bind(CORRIDOR_SOURCE_MANUAL)
        .push(", ")
        .push_bind(CORRIDOR_SOURCE_UNKNOWN)
        .push(")) AND object_id IN (");

    let mut sep = qb.separated(", ");
    for object_id in object_ids {
        sep.push_bind(object_id);
    }
    qb.push(")");

    qb.build_query_scalar::<String>().fetch_all(pool).await
}
pub async fn delete_collection_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_current_object_states_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<CollectionObjectState>, sqlx::Error> {
    sqlx::query_as::<_, (String, bool)>(
        r#"
        SELECT
            o.id,
            NOT (
                COALESCE(o.folder_path, '') LIKE 'DISABLED %'
                OR COALESCE(o.folder_path, '') LIKE '%/DISABLED %'
                OR COALESCE(o.folder_path, '') LIKE '%\\DISABLED %'
            ) AS is_enabled
        FROM objects o
        WHERE o.game_id = ?
        ORDER BY o.id
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(object_id, is_enabled)| CollectionObjectState {
                object_id,
                is_enabled,
            })
            .collect()
    })
}

// ── Batched Operations (O(N) Optimizations) ─────────────────

pub async fn batch_insert_collection_object_states(
    conn: &mut SqliteConnection,
    collection_id: &str,
    object_states: &[CollectionObjectState],
) -> Result<(), sqlx::Error> {
    if object_states.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::new(
        "INSERT OR REPLACE INTO collection_object_states (collection_id, object_id, is_enabled) ",
    );
    qb.push_values(object_states, |mut b, state| {
        b.push_bind(collection_id)
            .push_bind(&state.object_id)
            .push_bind(state.is_enabled);
    });

    qb.build().execute(conn).await?;
    Ok(())
}

pub async fn get_collection_roots(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionPreviewMod>, sqlx::Error> {
    if let Some(snapshot) = get_collection_snapshot(pool, collection_id).await? {
        return Ok(snapshot.roots);
    }

    let rows: Vec<(
        String,
        String,
        String,
        bool,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT
            root_path_key,
            display_name,
            root_path,
            COALESCE(is_safe, 1),
            object_id,
            object_name,
            object_type,
            root_kind
        FROM collection_roots
        WHERE collection_id = ?
        ORDER BY display_name, root_path
        "#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                root_path_key,
                display_name,
                root_path,
                is_safe,
                object_id,
                object_name,
                object_type,
                root_kind,
            )| CollectionPreviewMod {
                id: format!("collection-root:{root_path_key}"),
                actual_name: display_name,
                folder_path: root_path,
                is_safe,
                object_id,
                object_name,
                object_type,
                node_type: Some(root_kind),
            },
        )
        .collect())
}

pub async fn get_collection_signature(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let signature =
        sqlx::query_scalar::<_, Option<String>>("SELECT signature FROM collections WHERE id = ?")
            .bind(collection_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    if signature.is_some() {
        return Ok(signature);
    }

    sqlx::query_scalar("SELECT signature FROM collection_signatures WHERE collection_id = ?")
        .bind(collection_id)
        .fetch_optional(pool)
        .await
}

pub async fn find_named_collections_by_signature(
    pool: &SqlitePool,
    game_id: &str,
    is_safe_context: bool,
    signature: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT c.id, c.name
        FROM collections c
        WHERE c.game_id = ?
          AND c.is_safe_context = ?
          AND COALESCE(c.is_last_unsaved, 0) = 0
          AND c.signature = ?
        ORDER BY c.name ASC
        "#,
    )
    .bind(game_id)
    .bind(is_safe_context)
    .bind(signature)
    .fetch_all(pool)
    .await
}

pub async fn batch_get_mod_id_by_paths_pool(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    mods_path: Option<&str>,
) -> Result<HashMap<String, String>, sqlx::Error> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT folder_path_key, id FROM mods WHERE game_id = ");
    qb.push_bind(game_id).push(" AND folder_path_key IN (");

    let mut sep = qb.separated(", ");
    for path in paths {
        sep.push_bind(collection_mod_path_key(path, mods_path));
    }
    qb.push(")");

    let rows: Vec<(String, String)> = qb.build_query_as().fetch_all(pool).await?;

    let lookup: HashMap<String, String> = rows.into_iter().collect();
    let mut result = HashMap::new();
    for path in paths {
        let path_key = collection_mod_path_key(path, mods_path);
        if let Some(id) = lookup.get(&path_key) {
            result.insert(path.clone(), id.clone());
        }
    }
    Ok(result)
}
