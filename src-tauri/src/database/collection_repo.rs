use crate::services::collections::types::{
    Collection, CollectionDetails, CollectionPreviewMod, ModState,
};
use sqlx::{QueryBuilder, Sqlite, SqliteConnection, SqlitePool};
use std::collections::HashMap;

pub async fn list_collections(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<Vec<Collection>, sqlx::Error> {
    let sql = if safe_mode_enabled {
        r#"
        SELECT c.id, c.name, c.game_id, c.is_safe_context, COUNT(ci.mod_id) as member_count, COALESCE(c.is_last_unsaved, 0) as is_last_unsaved
        FROM collections c
        LEFT JOIN collection_items ci ON c.id = ci.collection_id
        WHERE c.game_id = ? AND c.is_safe_context = 1
        GROUP BY c.id
        ORDER BY c.is_last_unsaved DESC, c.name
        "#
    } else {
        r#"
        SELECT c.id, c.name, c.game_id, c.is_safe_context, COUNT(ci.mod_id) as member_count, COALESCE(c.is_last_unsaved, 0) as is_last_unsaved
        FROM collections c
        LEFT JOIN collection_items ci ON c.id = ci.collection_id
        WHERE c.game_id = ?
        GROUP BY c.id
        ORDER BY c.is_last_unsaved DESC, c.name
        "#
    };

    let rows = sqlx::query_as::<_, (String, String, String, bool, i64, bool)>(sql)
        .bind(game_id)
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
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM collections WHERE game_id = ? AND name = ? AND is_safe_context = ?",
    )
    .bind(game_id)
    .bind(name)
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
    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(game_id)
        .bind(is_safe_context)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_enabled_mod_ids(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM mods WHERE game_id = ? AND status = 'ENABLED'")
        .bind(game_id)
        .fetch_all(conn)
        .await
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

pub async fn insert_collection_item(
    conn: &mut SqliteConnection,
    collection_id: &str,
    mod_id: &str,
    mod_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO collection_items (collection_id, mod_id, mod_path) VALUES (?, ?, ?)",
    )
    .bind(collection_id)
    .bind(mod_id)
    .bind(mod_path)
    .execute(conn)
    .await?;
    Ok(())
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
    sqlx::query("UPDATE collections SET name = ? WHERE id = ? AND game_id = ?")
        .bind(name)
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

pub async fn get_collection_items(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<Vec<(String, Option<String>)>, sqlx::Error> {
    sqlx::query_as("SELECT mod_id, mod_path FROM collection_items WHERE collection_id = ?")
        .bind(collection_id)
        .fetch_all(conn)
        .await
}

pub async fn delete_collection_items(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collection_items WHERE collection_id = ?")
        .bind(collection_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_collection(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collections WHERE id = ? AND game_id = ?")
        .bind(id)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_collection_details(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<Option<CollectionDetails>, sqlx::Error> {
    let coll_row: Option<(String, String, String, bool, bool)> = sqlx::query_as(
        "SELECT id, name, game_id, is_safe_context, COALESCE(is_last_unsaved, 0) FROM collections WHERE id = ? AND game_id = ?",
    )
    .bind(id)
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    if let Some((cid, name, gid, safe, is_last_unsaved)) = coll_row {
        let mod_ids: Vec<String> = sqlx::query_scalar(
            "SELECT mod_id FROM collection_items WHERE collection_id = ? ORDER BY mod_id",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        Ok(Some(CollectionDetails {
            collection: Collection {
                id: cid,
                name,
                game_id: gid,
                is_safe_context: safe,
                member_count: mod_ids.len(),
                is_last_unsaved,
            },
            mod_ids,
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_collection_preview_mods(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
) -> Result<Vec<CollectionPreviewMod>, sqlx::Error> {
    let sql = r#"
        SELECT
            m.id,
            m.actual_name,
            m.folder_path,
            COALESCE(m.is_safe, 0) as is_safe,
            m.object_id,
            o.name as object_name,
            o.object_type
        FROM collection_items ci
        JOIN mods m ON ci.mod_id = m.id
        LEFT JOIN objects o ON m.object_id = o.id
        WHERE ci.collection_id = ? AND m.game_id = ?
        ORDER BY o.name ASC, m.actual_name ASC
    "#;

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            bool,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(sql)
    .bind(id)
    .bind(game_id)
    .fetch_all(pool)
    .await?;

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
                }
            },
        )
        .collect())
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

pub async fn get_mod_id_by_path(
    pool: &SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM mods WHERE folder_path = ? AND game_id = ?")
        .bind(folder_path)
        .bind(game_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_collection_item_mod_id(
    pool: &SqlitePool,
    collection_id: &str,
    old_mod_id: &str,
    new_mod_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE collection_items SET mod_id = ? WHERE collection_id = ? AND mod_id = ?")
        .bind(new_mod_id)
        .bind(collection_id)
        .bind(old_mod_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_object_ids_for_collection(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT DISTINCT object_id FROM mods WHERE id IN (SELECT mod_id FROM collection_items WHERE collection_id = ?) AND object_id IS NOT NULL")
        .bind(collection_id)
        .fetch_all(pool)
        .await
}

pub async fn delete_snapshot_collection(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM collection_items WHERE collection_id IN (SELECT id FROM collections WHERE game_id = ? AND is_last_unsaved = 1)")
        .bind(game_id)
        .execute(&mut *conn)
        .await?;

    sqlx::query("DELETE FROM collections WHERE game_id = ? AND is_last_unsaved = 1")
        .bind(game_id)
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
    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context, is_last_unsaved) VALUES (?, ?, ?, ?, 1)")
        .bind(snapshot_id)
        .bind(name)
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
        QueryBuilder::new("SELECT id, folder_path, status FROM mods WHERE game_id = ");
    qb.push_bind(game_id).push(" AND id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    qb.push(")");

    let rows: Vec<(String, String, String)> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, folder_path, status)| ModState {
            id,
            folder_path,
            status,
        })
        .collect())
}

pub async fn get_enabled_conflicting_mod_states(
    pool: &SqlitePool,
    game_id: &str,
    target_ids: &[String],
    object_ids: &[String],
) -> Result<Vec<ModState>, sqlx::Error> {
    if object_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id, folder_path, status FROM mods WHERE game_id = ");
    qb.push_bind(game_id)
        .push(" AND status = 'ENABLED' AND object_id IN (");

    let mut object_separated = qb.separated(", ");
    for object_id in object_ids {
        object_separated.push_bind(object_id);
    }
    qb.push(")");

    if !target_ids.is_empty() {
        qb.push(" AND id NOT IN (");
        let mut id_separated = qb.separated(", ");
        for id in target_ids {
            id_separated.push_bind(id);
        }
        qb.push(")");
    }

    let rows: Vec<(String, String, String)> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, folder_path, status)| ModState {
            id,
            folder_path,
            status,
        })
        .collect())
}

pub async fn batch_update_mods_status_and_path(
    pool: &SqlitePool,
    updates: &[(String, String, String)], // (id, status, folder_path)
) -> Result<(), sqlx::Error> {
    if updates.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (id, status, folder_path) in updates {
        sqlx::query("UPDATE mods SET status = ?, folder_path = ? WHERE id = ?")
            .bind(status)
            .bind(folder_path)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn get_last_unsaved_collection_id_and_safe_context(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Option<(String, bool)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, is_safe_context FROM collections WHERE game_id = ? AND is_last_unsaved = 1",
    )
    .bind(game_id)
    .fetch_optional(conn)
    .await
}

pub async fn get_collection_item_mod_ids(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT mod_id FROM collection_items WHERE collection_id = ?")
        .bind(collection_id)
        .fetch_all(conn)
        .await
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

pub async fn update_collection_item_mod_id_global(
    conn: &mut SqliteConnection,
    old_mod_id: &str,
    new_mod_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE collection_items SET mod_id = ? WHERE mod_id = ?")
        .bind(new_mod_id)
        .bind(old_mod_id)
        .execute(conn)
        .await?;
    Ok(())
}
