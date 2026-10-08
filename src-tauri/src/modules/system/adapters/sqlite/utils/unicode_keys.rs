use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::HashMap;

use crate::modules::system::adapters::sqlite::settings;
use crate::shared::path_key::{canonical_name_key, folder_path_key};

const UNICODE_KEY_VERSION_KEY: &str = "unicode_key_version";
const UNICODE_KEY_VERSION: &str = "1";

pub async fn ensure_unicode_keys(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let current = settings::get_setting(&mut *tx, UNICODE_KEY_VERSION_KEY).await?;
    if current.as_deref() == Some(UNICODE_KEY_VERSION) {
        tx.commit().await?;
        return Ok(());
    }

    let game_mod_paths = load_game_mod_paths(&mut tx).await?;

    backfill_mod_keys(&mut tx, &game_mod_paths).await?;
    backfill_object_keys(&mut tx, &game_mod_paths).await?;
    backfill_collection_keys(&mut tx).await?;

    settings::set_setting(&mut *tx, UNICODE_KEY_VERSION_KEY, UNICODE_KEY_VERSION).await?;
    tx.commit().await?;
    Ok(())
}

async fn load_game_mod_paths(
    conn: &mut SqliteConnection,
) -> Result<HashMap<String, Option<String>>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, mods_path FROM games")
        .fetch_all(&mut *conn)
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn legacy_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrated schema");
        sqlx::query("INSERT INTO games (id, name, game_type, path, mods_path) VALUES ('game', 'Game', ?, 'C:/Game', 'C:/Game/Mods')")
            .bind(crate::modules::games::domain::models::GameType::GIMI as i64)
            .execute(&pool).await.expect("game fixture");
        sqlx::query("INSERT INTO objects (id, game_id, name, folder_path, object_type, name_key, folder_path_key) VALUES ('object', 'game', 'Object', 'Object', 'Character', 'legacy-name', 'legacy-object')")
            .execute(&pool).await.expect("object fixture");
        sqlx::query("INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key) VALUES ('mod', 'game', 'object', 'Mod', 'Object/Mod', 'legacy-mod')")
            .execute(&pool).await.expect("mod fixture");
        pool
    }

    #[tokio::test]
    async fn backfill_persists_its_version_in_the_migrated_schema() {
        let pool = legacy_pool().await;
        ensure_unicode_keys(&pool).await.expect("backfill");
        assert_eq!(
            settings::get_setting(&pool, UNICODE_KEY_VERSION_KEY)
                .await
                .expect("version read")
                .as_deref(),
            Some(UNICODE_KEY_VERSION)
        );
    }

    #[tokio::test]
    async fn repeated_startup_does_not_rewrite_index_keys() {
        let pool = legacy_pool().await;
        ensure_unicode_keys(&pool).await.expect("first startup");
        let before: i64 = sqlx::query_scalar("SELECT total_changes()")
            .fetch_one(&pool)
            .await
            .expect("change count");
        ensure_unicode_keys(&pool).await.expect("reopened startup");
        let after: i64 = sqlx::query_scalar("SELECT total_changes()")
            .fetch_one(&pool)
            .await
            .expect("change count");
        assert_eq!(
            after, before,
            "reopening an indexed database must perform no backfill writes"
        );
    }

    #[tokio::test]
    async fn version_write_failure_rolls_back_the_backfill() {
        let pool = legacy_pool().await;
        sqlx::query("CREATE TRIGGER reject_unicode_version BEFORE INSERT ON app_settings WHEN NEW.key = 'unicode_key_version' BEGIN SELECT RAISE(ABORT, 'injected version failure'); END")
            .execute(&pool).await.expect("failure injection");
        assert!(ensure_unicode_keys(&pool).await.is_err());
        let key: String = sqlx::query_scalar("SELECT folder_path_key FROM mods WHERE id = 'mod'")
            .fetch_one(&pool)
            .await
            .expect("unchanged mod key");
        assert_eq!(key, "legacy-mod");
    }

    #[tokio::test]
    #[ignore = "manual startup backfill timing comparison on a 5,000-mod fixture"]
    async fn benchmark_reopened_index_backfill() {
        const MOD_COUNT: i64 = 5_000;
        const SAMPLE_COUNT: usize = 5;

        let pool = legacy_pool().await;
        sqlx::query("WITH RECURSIVE entries(n) AS (SELECT 2 UNION ALL SELECT n + 1 FROM entries WHERE n < ?) INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key) SELECT 'mod-' || n, 'game', 'object', 'Mod ' || n, 'Object/Mod ' || n, 'legacy-' || n FROM entries")
            .bind(MOD_COUNT)
            .execute(&pool).await.expect("large library fixture");
        ensure_unicode_keys(&pool).await.expect("initial index");

        let mut missing_marker_us = Vec::with_capacity(SAMPLE_COUNT);
        let mut durable_marker_us = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            // Reproduce the old missing-marker condition with the same backfill
            // and database; exclude marker deletion from the measured interval.
            settings::delete_setting(&pool, UNICODE_KEY_VERSION_KEY)
                .await
                .expect("missing marker");
            let before: i64 = sqlx::query_scalar("SELECT total_changes()")
                .fetch_one(&pool)
                .await
                .expect("write count");
            let start = std::time::Instant::now();
            ensure_unicode_keys(&pool)
                .await
                .expect("missing-marker startup");
            missing_marker_us.push(start.elapsed().as_micros());
            let after: i64 = sqlx::query_scalar("SELECT total_changes()")
                .fetch_one(&pool)
                .await
                .expect("write count");
            assert!(after - before >= MOD_COUNT);

            let start = std::time::Instant::now();
            ensure_unicode_keys(&pool)
                .await
                .expect("durable-marker startup");
            durable_marker_us.push(start.elapsed().as_micros());
            let after_skip: i64 = sqlx::query_scalar("SELECT total_changes()")
                .fetch_one(&pool)
                .await
                .expect("write count");
            assert_eq!(after_skip, after, "reopened startup must not rewrite rows");
        }
        println!("5,000-mod startup backfill: missing_marker_us={missing_marker_us:?} durable_marker_us={durable_marker_us:?}");
    }
}
