use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::HashMap;

use crate::common::path_key::{canonical_name_key, folder_path_key};
use crate::repo::settings_repo;

const UNICODE_KEY_VERSION_KEY: &str = "unicode_key_version";
const UNICODE_KEY_VERSION: &str = "1";

pub async fn ensure_unicode_keys(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let current = settings_repo::get_app_meta(pool, UNICODE_KEY_VERSION_KEY).await;
    if current.as_deref() == Some(UNICODE_KEY_VERSION) {
        return Ok(());
    }

    let game_mod_paths = load_game_mod_paths(pool).await?;
    let mut tx = pool.begin().await?;

    backfill_mod_keys(&mut tx, &game_mod_paths).await?;
    backfill_object_keys(&mut tx, &game_mod_paths).await?;
    backfill_collection_keys(&mut tx).await?;

    tx.commit().await?;
    settings_repo::set_app_meta(pool, UNICODE_KEY_VERSION_KEY, UNICODE_KEY_VERSION).await;
    Ok(())
}

async fn load_game_mod_paths(
    pool: &SqlitePool,
) -> Result<HashMap<String, Option<String>>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, mods_path FROM games")
        .fetch_all(pool)
        .await?;

    let mut map = HashMap::new();
    for row in rows {
        let game_id: String = row.try_get("id")?;
        let mods_path: Option<String> = row.try_get("mods_path")?;
        map.insert(game_id, mods_path);
    }
    Ok(map)
}

async fn backfill_mod_keys(
    conn: &mut SqliteConnection,
    game_mod_paths: &HashMap<String, Option<String>>,
) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("SELECT id, game_id, folder_path FROM mods")
        .fetch_all(&mut *conn)
        .await?;

    for row in rows {
        let id: String = row.try_get("id")?;
        let game_id: String = row.try_get("game_id")?;
        let folder_path: String = row.try_get("folder_path")?;
        let mod_path = game_mod_paths
            .get(&game_id)
            .and_then(|value| value.as_deref());
        let path_key = folder_path_key(&folder_path, mod_path);

        sqlx::query("UPDATE mods SET folder_path_key = ? WHERE id = ?")
            .bind(path_key)
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}

async fn backfill_object_keys(
    conn: &mut SqliteConnection,
    game_mod_paths: &HashMap<String, Option<String>>,
) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("SELECT id, game_id, name, folder_path FROM objects")
        .fetch_all(&mut *conn)
        .await?;

    for row in rows {
        let id: String = row.try_get("id")?;
        let game_id: String = row.try_get("game_id")?;
        let name: String = row.try_get("name")?;
        let folder_path: Option<String> = row.try_get("folder_path")?;
        let mods_path = game_mod_paths
            .get(&game_id)
            .and_then(|value| value.as_deref());
        let name_key = canonical_name_key(&name);
        let folder_key = folder_path
            .as_deref()
            .map(|path| folder_path_key(path, mods_path));

        sqlx::query("UPDATE objects SET name_key = ?, folder_path_key = ? WHERE id = ?")
            .bind(name_key)
            .bind(folder_key)
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}

async fn backfill_collection_keys(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("SELECT id, name FROM collections")
        .fetch_all(&mut *conn)
        .await?;

    for row in rows {
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let name_key = canonical_name_key(&name);

        sqlx::query("UPDATE collections SET name_key = ? WHERE id = ?")
            .bind(name_key)
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}
