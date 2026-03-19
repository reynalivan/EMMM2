use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::HashMap;

use crate::database::settings_repo;
use crate::services::path_key::{
    collection_mod_path_key, collection_name_key, folder_path_key, object_name_key,
};

const UNICODE_KEY_VERSION_KEY: &str = "unicode_key_version";
const UNICODE_KEY_VERSION: &str = "1";

pub async fn ensure_unicode_keys(pool: &SqlitePool) -> Result<(), String> {
    let current = settings_repo::get_app_meta(pool, UNICODE_KEY_VERSION_KEY).await;
    if current.as_deref() == Some(UNICODE_KEY_VERSION) {
        return Ok(());
    }

    let game_mod_paths = load_game_mod_paths(pool).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    backfill_mod_keys(&mut tx, &game_mod_paths).await?;
    backfill_object_keys(&mut tx, &game_mod_paths).await?;
    backfill_collection_keys(&mut tx).await?;
    backfill_collection_item_keys(&mut tx, &game_mod_paths).await?;
    backfill_collection_nested_item_keys(&mut tx, &game_mod_paths).await?;
    ensure_unicode_key_indexes(&mut tx).await?;

    tx.commit().await.map_err(|e| e.to_string())?;
    settings_repo::set_app_meta(pool, UNICODE_KEY_VERSION_KEY, UNICODE_KEY_VERSION).await;
    Ok(())
}

async fn load_game_mod_paths(pool: &SqlitePool) -> Result<HashMap<String, Option<String>>, String> {
    let rows = sqlx::query("SELECT id, mod_path FROM games")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for row in rows {
        let game_id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let mod_path: Option<String> = row.try_get("mod_path").map_err(|e| e.to_string())?;
        map.insert(game_id, mod_path);
    }
    Ok(map)
}

async fn backfill_mod_keys(
    conn: &mut SqliteConnection,
    game_mod_paths: &HashMap<String, Option<String>>,
) -> Result<(), String> {
    let rows = sqlx::query("SELECT id, game_id, folder_path FROM mods")
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let game_id: String = row.try_get("game_id").map_err(|e| e.to_string())?;
        let folder_path: String = row.try_get("folder_path").map_err(|e| e.to_string())?;
        let mod_path = game_mod_paths
            .get(&game_id)
            .and_then(|value| value.as_deref());
        let path_key = folder_path_key(&folder_path, mod_path);

        sqlx::query("UPDATE mods SET folder_path_key = ? WHERE id = ?")
            .bind(path_key)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn backfill_object_keys(
    conn: &mut SqliteConnection,
    game_mod_paths: &HashMap<String, Option<String>>,
) -> Result<(), String> {
    let rows = sqlx::query("SELECT id, game_id, name, folder_path FROM objects")
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let game_id: String = row.try_get("game_id").map_err(|e| e.to_string())?;
        let name: String = row.try_get("name").map_err(|e| e.to_string())?;
        let folder_path: Option<String> = row.try_get("folder_path").map_err(|e| e.to_string())?;
        let mods_path = game_mod_paths
            .get(&game_id)
            .and_then(|value| value.as_deref());
        let name_key = object_name_key(&name);
        let folder_key = folder_path
            .as_deref()
            .map(|path| folder_path_key(path, mods_path));

        sqlx::query("UPDATE objects SET name_key = ?, folder_path_key = ? WHERE id = ?")
            .bind(name_key)
            .bind(folder_key)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn backfill_collection_keys(conn: &mut SqliteConnection) -> Result<(), String> {
    let rows = sqlx::query("SELECT id, name FROM collections")
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let name: String = row.try_get("name").map_err(|e| e.to_string())?;
        let name_key = collection_name_key(&name);

        sqlx::query("UPDATE collections SET name_key = ? WHERE id = ?")
            .bind(name_key)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn backfill_collection_item_keys(
    conn: &mut SqliteConnection,
    game_mod_paths: &HashMap<String, Option<String>>,
) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT ci.collection_id, ci.mod_id, ci.mod_path, c.game_id
         FROM collection_items ci
         JOIN collections c ON c.id = ci.collection_id
         WHERE ci.mod_path IS NOT NULL",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    for row in rows {
        let collection_id: String = row.try_get("collection_id").map_err(|e| e.to_string())?;
        let mod_id: Option<String> = row.try_get("mod_id").map_err(|e| e.to_string())?;
        let mod_path: String = row.try_get("mod_path").map_err(|e| e.to_string())?;
        let game_id: String = row.try_get("game_id").map_err(|e| e.to_string())?;
        let mods_path = game_mod_paths
            .get(&game_id)
            .and_then(|value| value.as_deref());
        let mod_path_key = collection_mod_path_key(&mod_path, mods_path);

        sqlx::query(
            "UPDATE collection_items SET mod_path_key = ?
             WHERE collection_id = ? AND ((mod_id = ?) OR (mod_id IS NULL AND ? IS NULL)) AND mod_path = ?",
        )
        .bind(mod_path_key)
        .bind(collection_id)
        .bind(mod_id.clone())
        .bind(mod_id)
        .bind(mod_path)
        .execute(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn backfill_collection_nested_item_keys(
    conn: &mut SqliteConnection,
    game_mod_paths: &HashMap<String, Option<String>>,
) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT cni.collection_id, cni.mod_path, c.game_id
         FROM collection_nested_items cni
         JOIN collections c ON c.id = cni.collection_id",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    for row in rows {
        let collection_id: String = row.try_get("collection_id").map_err(|e| e.to_string())?;
        let mod_path: String = row.try_get("mod_path").map_err(|e| e.to_string())?;
        let game_id: String = row.try_get("game_id").map_err(|e| e.to_string())?;
        let mods_path = game_mod_paths
            .get(&game_id)
            .and_then(|value| value.as_deref());
        let mod_path_key = collection_mod_path_key(&mod_path, mods_path);

        sqlx::query(
            "UPDATE collection_nested_items SET mod_path_key = ? WHERE collection_id = ? AND mod_path = ?",
        )
        .bind(mod_path_key)
        .bind(collection_id)
        .bind(mod_path)
        .execute(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn ensure_unicode_key_indexes(conn: &mut SqliteConnection) -> Result<(), String> {
    let statements = [
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_mods_folder_path_key_unique ON mods(game_id, folder_path_key)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_folder_path_key_unique ON objects(game_id, folder_path_key)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_name_key_unique ON objects(game_id, name_key)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_name_key_context_unique ON collections(game_id, name_key, is_safe_context)",
    ];

    for statement in statements {
        sqlx::query(statement)
            .execute(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
