//! Bulk operations, each committed in a single transaction.

use super::paths::get_game_mod_path;
use crate::shared::path_key::folder_path_key;
use sqlx::SqlitePool;
use std::collections::HashSet;

// SQLite builds commonly permit 999 bind variables. Favorite/pin queries bind
// the value and game id too, so each batch stays strictly below that ceiling.
const SQLITE_BIND_VARIABLE_LIMIT: usize = 999;
const FLAG_UPDATE_FIXED_BINDS: usize = 2;
const MAX_PATH_KEYS_PER_BATCH: usize = SQLITE_BIND_VARIABLE_LIMIT - FLAG_UPDATE_FIXED_BINDS - 1;

/// Delete the listed mods from one game. Paths are matched the way Disk
/// Reconcile stores them: relative to the mods root, keyed with no
/// `mods_path`. Two games can hold the same mod at the same relative path, so
/// the key alone is ambiguous and `game_id` is what makes the match
/// single-game.
#[cfg(test)]
pub async fn batch_delete_by_path(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query("DELETE FROM mods WHERE folder_path_key = ? AND game_id = ?")
            .bind(folder_path_key(path, None))
            .bind(game_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn batch_set_favorite(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    favorite: bool,
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    batch_set_boolean_flag(pool, game_id, paths, "is_favorite", favorite).await
}

pub async fn batch_set_pinned(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    pin: bool,
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    batch_set_boolean_flag(pool, game_id, paths, "is_pinned", pin).await
}

async fn batch_set_boolean_flag(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    column: &'static str,
    value: bool,
) -> Result<(), sqlx::Error> {
    let mods_path = get_game_mod_path(pool, game_id).await?;
    let path_keys = chunk_path_keys(
        paths
            .iter()
            .map(|path| folder_path_key(path, mods_path.as_deref()))
            .collect(),
    );
    if path_keys.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for path_key_batch in path_keys {
        let placeholders = std::iter::repeat("?")
            .take(path_key_batch.len())
            .collect::<Vec<_>>()
            .join(", ");
        let statement = format!(
            "UPDATE mods SET {column} = ? WHERE game_id = ? AND folder_path_key IN ({placeholders})"
        );
        let mut query = sqlx::query(&statement).bind(value).bind(game_id);
        for path_key in path_key_batch {
            query = query.bind(path_key);
        }
        query.execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

fn chunk_path_keys(path_keys: Vec<String>) -> Vec<Vec<String>> {
    let mut seen = HashSet::with_capacity(path_keys.len());
    let unique = path_keys
        .into_iter()
        .filter(|path_key| seen.insert(path_key.clone()))
        .collect::<Vec<_>>();
    unique
        .chunks(MAX_PATH_KEYS_PER_BATCH)
        .map(<[String]>::to_vec)
        .collect()
}

pub async fn batch_set_safety(
    pool: &SqlitePool,
    game_id: &str,
    paths: &[String],
    safe: bool,
) -> Result<(), sqlx::Error> {
    if paths.is_empty() {
        return Ok(());
    }

    let mods_path = get_game_mod_path(pool, game_id).await?;
    let mut tx = pool.begin().await?;
    for path in paths {
        sqlx::query(
            "UPDATE mods SET is_safe = ?, safety_source = ? WHERE folder_path_key = ? AND game_id = ?",
        )
        .bind(safe)
        .bind(crate::shared::safety_constants::SAFETY_SOURCE_MANUAL)
        .bind(folder_path_key(path, mods_path.as_deref()))
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{batch_set_favorite, chunk_path_keys, MAX_PATH_KEYS_PER_BATCH};

    #[test]
    fn chunking_deduplicates_keys_and_stays_below_the_bind_limit() {
        let keys = (0..(MAX_PATH_KEYS_PER_BATCH + 3))
            .map(|index| format!("mod/{index}"))
            .chain(std::iter::once("mod/0".to_string()))
            .collect::<Vec<_>>();

        let batches = chunk_path_keys(keys);

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), MAX_PATH_KEYS_PER_BATCH);
        assert_eq!(batches[1].len(), 3);
        assert_eq!(
            batches
                .iter()
                .flatten()
                .filter(|key| *key == "mod/0")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn favorite_batch_deduplicates_paths_and_stays_scoped_to_one_game() {
        let context = crate::test_utils::init_test_db().await;
        for (game_id, mods_path) in [("game-a", "C:/Mods/A"), ("game-b", "C:/Mods/B")] {
            crate::test_utils::insert_test_game(
                &context.pool,
                &crate::test_utils::TestGameFixture {
                    id: game_id,
                    name: game_id,
                    game_type: crate::modules::games::domain::models::GameType::GIMI,
                    path: mods_path,
                    mods_path: Some(mods_path),
                },
            )
            .await
            .expect("insert test game");
        }
        for (id, game_id, mods_path) in [
            ("a-one", "game-a", "C:/Mods/A"),
            ("a-two", "game-a", "C:/Mods/A"),
            ("b-one", "game-b", "C:/Mods/B"),
        ] {
            crate::test_utils::insert_test_mod(
                &context.pool,
                &crate::test_utils::TestModFixture {
                    id,
                    game_id,
                    object_id: None,
                    actual_name: id,
                    folder_path: if id == "a-two" { "Two" } else { "One" },
                    status: crate::modules::games::domain::models::ItemStatus::Enabled,
                    is_safe: true,
                    object_type: Some("Other"),
                    mods_path: Some(mods_path),
                },
            )
            .await
            .expect("insert test mod");
        }

        batch_set_favorite(
            &context.pool,
            "game-a",
            &["One".to_string(), "One".to_string(), "Two".to_string()],
            true,
        )
        .await
        .expect("batch favorite");

        let favorites: Vec<(String, bool)> =
            sqlx::query_as("SELECT id, is_favorite FROM mods ORDER BY id")
                .fetch_all(&context.pool)
                .await
                .expect("read favorites");
        assert_eq!(
            favorites,
            vec![
                ("a-one".to_string(), true),
                ("a-two".to_string(), true),
                ("b-one".to_string(), false),
            ]
        );
    }
}
